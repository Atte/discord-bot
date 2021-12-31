let pkgs = import <nixpkgs-unstable> { };
in pkgs.mkShell {
    buildInputs = with pkgs; [ cargo rustc rustfmt rls ];

    RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
}