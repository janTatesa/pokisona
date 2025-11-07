{
  description = "A Nix-flake-based Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
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
              pkgs.rust-bin.nightly.latest.rustfmt
              (pkgs.rust-bin.stable.latest.default.override {
                extensions = [
                  "rust-analyzer"
                  "rust-src"
                ];
              })

            ];

            env = {
              ICED_BACKEND = "wgpu";
              RUSTFLAGS = "-C link-arg=-Wl,-rpath,${
                pkgs.lib.makeLibraryPath (
                  with pkgs;
                  [
                    wayland
                    libxkbcommon
                    vulkan-loader
                  ]
                )
              }";
            };
          };
        }
      );
    };
}
