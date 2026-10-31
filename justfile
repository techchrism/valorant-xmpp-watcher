default:
    @just --list

update:
    npins update

build:
    nix-build --no-out-link
