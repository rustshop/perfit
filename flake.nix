{
  description = "Rust + htmx + tailwind + nix + redb metric tracking service";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
    flake-utils.url = "github:numtide/flake-utils";
    flakebox = {
      url = "github:rustshop/flakebox?rev=84304c4690f11e225287e3cc042281cbeb34d9a3";
    };
  };

  outputs = { self, nixpkgs, flake-utils, flakebox }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        projectName = "perfit";

        pkgs = nixpkgs.legacyPackages.${system};

        flakeboxLib = flakebox.lib.${system} {
          config = {
            github.ci.buildOutputs = [ ".#ci.${projectName}" ];
            just.importPaths = [
              "justfile.custom"
            ];
          };
        };

        buildPaths = [
          "Cargo.toml"
          "Cargo.lock"
          ".cargo"
          "src"
          "assets"
          "build.rs"
          "tailwind.config.js"
        ];

        buildSrc = flakeboxLib.filterSubPaths {
          root = builtins.path {
            name = projectName;
            path = ./.;
          };
          paths = buildPaths;
        };

        multiBuild =
          (flakeboxLib.craneMultiBuild { }) (craneLib':
            let
              craneLib = (craneLib'.overrideArgs {
                pname = projectName;
                src = buildSrc;
                nativeBuildInputs = [ pkgs.tailwindcss ];
              });
            in
            {
              perfitd = craneLib.buildPackage {
                meta.mainProgram = "perfitd";

                preBuild = ''
                  export PERFITD_BUILD_OUT_DIR=$out/share
                '';
              };

              perfit = craneLib.buildPackage {
                meta.mainProgram = "perfit";

                preBuild = ''
                  export PERFITD_BUILD_OUT_DIR=$out/share
                '';
              };
            });
      in
      {
        packages = {
          default = multiBuild.perfit;
          perfit = multiBuild.perfit;
          perfitd = multiBuild.perfitd;
        };

        legacyPackages = multiBuild;

        devShells = flakeboxLib.mkShells {
          packages = [ ];
          nativeBuildInputs = [ pkgs.tailwindcss pkgs.cargo-insta ];
        };
      }
    );
}
