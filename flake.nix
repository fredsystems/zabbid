{
  description = "Dev shell and Linting";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    precommit = {
      url = "github:FredSystems/pre-commit-checks";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      precommit,
      ...
    }:
    let
      systems = precommit.lib.supportedSystems;
      inherit (nixpkgs) lib;
    in
    {
      ##########################################################################
      ## PACKAGES
      ##########################################################################
      packages = lib.genAttrs systems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        rec {
          zab-bid = pkgs.rustPlatform.buildRustPackage {
            pname = "zab-bid";
            version = "0.1.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = [
              pkgs.pkg-config
            ];

            meta = with pkgs.lib; {
              description = "Bidding tool";
              homepage = "https://github.com/fredsystems/zabbid";
              license = licenses.mit;
              platforms = platforms.linux;
              maintainers = [ maintainers.fredclausen ];
            };
          };

          default = zab-bid;
        }
      );

      ##########################################################################
      ## APPS (nix run .)
      ##########################################################################
      apps = lib.genAttrs systems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/zabbid";
        };
      });

      ##########################################################################
      ## CHECKS
      ##########################################################################
      checks = lib.genAttrs systems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          pre-commit = precommit.lib.mkCheck {
            inherit system;

            src = ./.;

            check_rust = true;
            check_docker = false;
            check_python = true;
            check_javascript = true;

            javascript = {
              enableBiome = true;
              enableTsc = true;
              tsConfig = "ui/tsconfig.json";
            };

            enableXtask = true;
            extraLibPathPkgs = [
              pkgs.mariadb-connector-c
            ];

            extraExcludes = [
              ".dictionary.txt"
              "typos.toml"
            ];
          };
        }
      );

      ##########################################################################
      ## DEV SHELLS
      ##########################################################################
      devShells = lib.genAttrs systems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          chk = self.checks.${system}.pre-commit;
        in
        {
          default = pkgs.mkShell {
            packages =
              with pkgs;
              [
                markdownlint-cli2
                cargo-deny
                cargo-machete
                typos
                cargo-llvm-cov
                sqlite
                nodejs
                diesel-cli
                docker
                mariadb.client
                mariadb
                pkg-config
                mariadb-connector-c
              ]
              ++ (chk.passthru.devPackages or [ ])
              ++ chk.enabledPackages;

            shellHook = ''
              # Run git-hooks.nix / pre-commit setup
              ${chk.shellHook}

              # Your own extras
              alias pre-commit="pre-commit run --all-files"
              alias xtask="cargo run -p xtask --"
            '';
          };
        }
      );

    };
}
