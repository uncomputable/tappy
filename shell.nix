with (import <nixpkgs> {});
let
  elementsd-simplicity = elementsd.overrideAttrs (_: rec {
    version = "unstable-2023-04-18";
    src = fetchFromGitHub {
      owner = "ElementsProject";
      repo = "elements";
      rev = "e144fa06f7ffdea88af20df7de91947fd57348ac"; # <-- update this to latest `simplicity` branch
      sha256 = "ooe+If3HWaJWpr2ux7DpiCTqB9Hv+aXjquEjplDjvhM="; # <-- ignore this
    };
  });
in
  mkShell {
    buildInputs = [
      elementsd-simplicity
    ];
  }
