# Yet Another Development Tool

WIP

This is a really stupid way of setting up arbitrary container environments by inserting arbitrary packages. It's made possibly because nix can get a closure of a package (the package and all recursive dependencies), which are created in a setup container and moved into the actual container where they are effectively treated as universal linux binaries.

