{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pest-ide-tools = {
      url = "github:janTatesa/pest-ide-tools";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
      };
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
            "x86_64-darwin"
            "aarch64-darwin"
          ]
          (
            system:
            f {
              pkgs = import inputs.nixpkgs {
                inherit system;
                overlays = [
                  inputs.rust-overlay.overlays.default
                  (final: prev: {
                    pest-ide-tools = inputs.pest-ide-tools.packages.${system}.pest-ide-tools;
                    rustfmt = prev.rust-bin.stable.latest.rustfmt;
                    rustToolchain = prev.rust-bin.stable.latest.default.override {
                      extensions = [
                        "rust-analyzer"
                        "rust-src"
                      ];
                    };
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
              pest-ide-tools
            ];
          };
        }
      );
    };
}
