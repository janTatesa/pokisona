{
  description = "A Nix-flake-based Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    # pest-ide-tools.url = "github:janTatesa/pest-ide-tools";
  };

  outputs =
    inputs:
    let
      forEachSupportedSystem =
        f:
        inputs.nixpkgs.lib.genAttrs
          [
            "x86_64-linux"
            "aarch64-linux"
          ]
          (
            system:
            f {
              pkgs = import inputs.nixpkgs {
                inherit system;
                overlays = [
                  inputs.rust-overlay.overlays.default
                  (final: prev: {
                    rustToolchain = prev.rust-bin.stable.latest.default.override {
                      extensions = [
                        "rust-analyzer"
                        "rust-src"
                      ];
                    };
                    rustfmt = prev.lib.hiPrio prev.rust-bin.nightly.latest.rustfmt;
                    deps = with prev; [
                      wayland
                      libxkbcommon
                      vulkan-loader
                    ];
                  })
                ];
              };
            }
          );
    in
    {
      devShells = forEachSupportedSystem (
        { pkgs }:
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              pkg-config
              rustfmt
              rustToolchain
              openssl
            ];

            env = {
              ICED_BACKEND = "wgpu";
              RUSTFLAGS = "-C link-arg=-Wl,-rpath,${pkgs.lib.makeLibraryPath pkgs.deps}";
            };
          };
        }
      );
    };
}
