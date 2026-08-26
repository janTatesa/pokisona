use std::{
    collections::{BTreeSet, HashMap},
    env,
    fmt::{Debug, Display, Formatter},
    fs::{self, File},
    io::{self, Write},
    ops::Deref,
    str::FromStr,
    time::SystemTime
};

use chumsky::container::Container;
use jiff::{
    Zoned,
    civil::{Date, DateTime}
};
use log::{error, warn};
use serde::{Deserialize, Serialize};

use crate::markdown::Markdown;

#[derive(Serialize, Deserialize)]
pub struct Cache {
    ideas: HashMap<IdeaRef, CacheEntry>,
    last_modified: SystemTime
}

impl Deref for Cache {
    type Target = HashMap<IdeaRef, CacheEntry>;

    fn deref(&self) -> &Self::Target {
        &self.ideas
    }
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
pub struct CacheEntry {
    pub links: BTreeSet<IdeaRef>,
    pub backlinks: BTreeSet<IdeaRef>,
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
        let ideas = fs::read_dir(env::current_dir()?)?.filter_map(|entry| {
            let file = match entry {
                Ok(file) => file,
                Err(file) => {
                    // TODO: this isnt displayed in the app
                    error!("Error reading dir entry: {file}");
                    return None;
                }
            };

            let idea = IdeaRef::from_str(file.file_name().to_str()?.strip_suffix(".md")?).ok()?;
            let accessed = file.metadata().ok()?.accessed().ok()?;
            Some((idea, accessed))
        });

        let mut this = Cache {
            ideas: HashMap::new(),
            last_modified: SystemTime::now()
        };
        for (idea, creation) in ideas {
            let links = Markdown::new(&fs::read_to_string(format!("{idea}.md"))?)
                .links()
                .filter_map(|link| {
                    this.ideas.get_mut(&link)?.backlinks.push(idea);
                    Some(link)
                })
                .collect();
            let entry = CacheEntry {
                links,
                backlinks: BTreeSet::new(),
                last_accessed: creation
            };
            this.ideas.insert(idea, entry);
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

        let links = markdown
            .links()
            .filter_map(|link| {
                self.ideas.get_mut(&link)?.backlinks.push(idea);
                Some(link)
            })
            .collect();

        let mut file = File::create(format!("{idea}.md"))?;
        file.write_all(contents.as_bytes())?;
        let mut permissions = file.metadata()?.permissions();
        permissions.set_readonly(true);
        file.set_permissions(permissions)?;

        let entry = CacheEntry {
            links,
            backlinks: BTreeSet::new(),
            last_accessed: SystemTime::now()
        };
        self.ideas.insert(idea, entry);
        self.save()?;

        Ok(idea)
    }
}
