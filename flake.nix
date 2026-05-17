{
  description = "Nix flake for miq-v2";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }: 
    let
			system = "x86_64-linux";
		  pkgs = import nixpkgs { inherit system; };
		in
	  {
		  devShells.${system}.default = pkgs.mkShell rec {
        buildInputs = with pkgs; [ 
				  cargo
          rustc
          rustfmt
          clippy

          # Needed to point whatever this is at alsa-lib
          pkg-config

          # GUI libraries
          libxkbcommon
          libGL
          fontconfig

          # Wayland libraries
          wayland

          # MIDI libraries
          alsa-lib
				];

        LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
		  };
    };
}
