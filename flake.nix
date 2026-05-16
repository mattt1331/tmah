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

          # GUI libraries
          libxkbcommon
          libGL
          fontconfig

          # Wayland libraries
          wayland
				];

        LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
		  };
    };
}
