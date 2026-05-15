{
  description = "Nix flake for miq-v2";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-25.11";
  };

  outputs = { self, nixpkgs }: 
    let
			system = "x86_64-linux";
		  pkgs = import nixpkgs { inherit system; };
		in
	  {
		  devShells.${system}.default = pkgs.mkShell /*rec*/ {
        buildInputs = [ 
				  pkgs.cargo pkgs.rustc pkgs.rustfmt pkgs.clippy
				];
		  };
    };
}
