{
  inputs = {
    flake-utils.url = github:numtide/flake-utils;
    naersk.url = github:nix-community/naersk;
    nixpkgs.url = github:NixOS/nixpkgs/nixpkgs-unstable;
  };
  outputs = { self, flake-utils, naersk, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        version = "1.0.0";
        pkgs = (import nixpkgs) { inherit system; };
        naersk' = pkgs.callPackage naersk { };
        buildInputs = with pkgs; [];
        mkPackage = { name, buildInputs ? [ ] }: naersk'.buildPackage {
          cargoBuildOptions = opts: opts ++ [ "--package" name ];
          inherit buildInputs;
          inherit name;
          inherit version;
          nativeBuildInputs = with pkgs;[ cmake pkgconfig ];
          src = ./.;
        };
      in
      rec {
        formatter = nixpkgs.legacyPackages.${system}.nixpkgs-fmt;
        packages.matlab-beautifier = mkPackage { name = "matlab-beautifier"; };
        packages.default = packages.matlab-beautifier;
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo ];
          inherit buildInputs;
        };
      }
    );
}