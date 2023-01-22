{pkgs, ...}: {
  # https://devenv.sh/basics/
  env.GREET = "devenv";

  # https://devenv.sh/packages/
  packages = with pkgs; [
    alejandra
    cmake
    freetype
    git
    poppler.dev
    pkgconfig
  ];

  enterShell = ''
    git --version
  '';

  languages.rust = {
    enable = true;
    # https://devenv.sh/reference/options/#languagesrustversion
    version = "latest";
  };

  pre-commit.hooks = {
    alejandra.enable = true;
    clippy.enable = true;
    rustfmt.enable = true;
  };
}
