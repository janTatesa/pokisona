use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    env,
    fmt::{Debug, Display, Formatter},
    fs::{self, File},
    io::{self, Write},
    str::FromStr,
    time::SystemTime
};

use catppuccin::PALETTE;
use chumsky::container::Container;
use jiff::{
    Zoned,
    civil::{Date, DateTime}
};
use log::{error, warn};
use serde::{Deserialize, Serialize};

use crate::markdown::{Markdown, MarkdownSpan};

#[derive(Serialize, Deserialize)]
pub struct Cache {
    ideas: BTreeMap<IdeaRef, IdeaCache>,
    tags: HashMap<String, TagCache>,
    last_modified: SystemTime
}

#[derive(Serialize, Deserialize)]
pub struct TagCache {
    pub ideas: BTreeSet<IdeaRef>,
    pub color: catppuccin::ColorName
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct IdeaRef {
    date: Date,
    hour: i8,
    minute: i8,
    second: i8
}

impl FromStr for IdeaRef {
    type Err = <DateTime as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let date_time = DateTime::from_str(s)?;
        Ok(Self {
            date: date_time.date(),
            hour: date_time.hour(),
            minute: date_time.minute(),
            second: date_time.second()
        })
    }
}

impl Display for IdeaRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}T{:02}:{:02}:{:02}",
            self.date, self.hour, self.minute, self.second
        )
    }
}

#[derive(Deserialize, Serialize)]
pub struct IdeaCache {
    pub links: BTreeSet<IdeaRef>,
    pub backlinks: BTreeSet<IdeaRef>,
    pub tags: BTreeSet<String>,
    pub last_accessed: SystemTime
}

impl Cache {
    const PATH: &str = ".pokisona/cache.bin";

    pub fn load() -> anyhow::Result<Self> {
        let this: Self = postcard::from_bytes(&fs::read(Self::PATH)?)?;
        if this.last_modified < fs::metadata(env::current_dir()?)?.modified()? {
            warn!("Vault has been modified by external process, rebuilding cache");
            Self::new()
        } else {
            Ok(this)
        }
    }

    pub fn read_idea(&mut self, idea: IdeaRef) -> io::Result<String> {
        let path = format!("{idea}.md");
        let contents = fs::read_to_string(&path)?;
        self.ideas.get_mut(&idea).unwrap().last_accessed = fs::metadata(&path)?.accessed()?;
        self.save()?;
        Ok(contents)
    }

    pub fn new() -> anyhow::Result<Self> {
        let ideas: BTreeSet<_> = fs::read_dir(env::current_dir()?)?
            .filter_map(|entry| {
                let file = match entry {
                    Ok(file) => file,
                    Err(file) => {
                        // TODO: this isnt displayed in the app
                        error!("Error reading dir entry: {file}");
                        return None;
                    }
                };

                let idea =
                    IdeaRef::from_str(file.file_name().to_str()?.strip_suffix(".md")?).ok()?;
                let accessed = file.metadata().ok()?.accessed().ok()?;
                Some((idea, accessed))
            })
            .collect();

        let mut this = Cache {
            ideas: BTreeMap::new(),
            last_modified: SystemTime::now(),
            tags: HashMap::new()
        };
        for (idea, accessed) in ideas {
            let markdown = Markdown::new(&fs::read_to_string(format!("{idea}.md"))?);
            this.insert(idea, &markdown, accessed);
        }

        this.save()?;

        Ok(this)
    }

    fn save(&mut self) -> io::Result<()> {
        self.last_modified = SystemTime::now();
        fs::write(Self::PATH, postcard::to_allocvec(&self).unwrap())?;
        let vault = File::open(env::current_dir()?)?;
        vault.set_modified(self.last_modified)
    }

    pub fn create(&mut self, contents: &str, markdown: &Markdown) -> io::Result<IdeaRef> {
        let now = Zoned::now();
        let idea = IdeaRef {
            date: now.date(),
            hour: now.hour(),
            minute: now.minute(),
            second: now.second()
        };

        let mut file = File::create(format!("{idea}.md"))?;
        file.write_all(contents.as_bytes())?;
        let mut permissions = file.metadata()?.permissions();
        permissions.set_readonly(true);
        file.set_permissions(permissions)?;

        self.insert(idea, markdown, now.timestamp().into());
        self.save()?;

        Ok(idea)
    }

    fn insert(&mut self, idea: IdeaRef, markdown: &Markdown, last_accessed: SystemTime) {
        let mut links = BTreeSet::new();
        let mut tags = BTreeSet::new();
        for span in markdown.lines().iter().flat_map(|line| &line.spans) {
            match span {
                MarkdownSpan::Link { target, .. } => {
                    let Some(refered_idea) = self.ideas.get_mut(target) else {
                        continue;
                    };
                    refered_idea.backlinks.push(idea);
                    links.push(*target);
                }
                MarkdownSpan::Tag(tag) => {
                    let len = self.tags.len();
                    self.tags
                        .entry(tag.clone())
                        .or_insert_with(|| TagCache {
                            color: PALETTE.frappe.into_iter().nth(len % 12).unwrap().name,
                            ideas: BTreeSet::new()
                        })
                        .ideas
                        .push(idea);
                    tags.insert(tag.clone());
                }
                _ => {}
            }
        }

        let entry = IdeaCache {
            links,
            backlinks: BTreeSet::new(),
            last_accessed,
            tags
        };
        self.ideas.insert(idea, entry);
    }

    pub fn ideas(&self) -> &BTreeMap<IdeaRef, IdeaCache> {
        &self.ideas
    }

    pub fn tags(&self) -> &HashMap<String, TagCache> {
        &self.tags
    }
}
