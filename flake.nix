{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
      in {
        packages.default = naersk-lib.buildPackage {
          src = ./.;
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ openssl ];
        };
        devShells = with pkgs; {
          default = mkShell {
            buildInputs = [
              openssl
              pkg-config
              cargo
              rustc
              rustfmt
              pre-commit
              rustPackages.clippy
              rust-analyzer
            ];
            RUST_SRC_PATH = rustPlatform.rustLibSrc;
          };

          dev = mkShell {
            buildInputs = [ openssl pkg-config cargo ];
            shellHook = ''
              testenv=$(mktemp -d --suffix "eleanor-server")
              cargo build -Z unstable-options --out-dir $testenv

              echo -e 'Built eleanor-server\n'

              cd $testenv
              cat << EOF >> settings.toml
              port = 8008

              [[sources]]
              id = 0
              path = "~/Music"
              EOF

              echo -e 'Generated settings.toml with source 0 pointing to `~/Music`\n'

              ./eleanor-server user add test password

              echo -e '\nAdded user with credentials `test:password`\n'
              echo -e 'Running eleanor-server\n'

              exec ./eleanor-server
            '';
          };
        };
      });
}
