MODEL PACKS OF THE REFERENCE CONVERTER
======================================

This folder holds further test models for the reference converter (the
library CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference), beside the
GraphicsDemo sample's model packs, which the library's project file links from
the sample. Both are copied into the build output as
gltf-reference-models/<pack>/..., and GltfReferenceFolder.Write turns every
model file (.glb, .gltf) of every pack into cases of the reference folder that
the Rust glTF loader's tests read.

THE RULE FOR A PACK
  - Every pack is ONE folder here (Models/<pack>/), named in lower case, not
    the name of one of the sample's packs.
  - It holds the model files EXACTLY AS OBTAINED (the .glb / .gltf files with
    their .bin files and images, in the folders the source ships them in, so
    that every relative URI still resolves), never edited.
  - It holds a LICENSE file (or the licence text file the source ships).
  - It holds a SOURCE.txt that records, for each file: where it came from
    (the URL), its licence identified as CC0 or CC-BY (for CC-BY the
    attribution line exactly as the source asks for it), and the date it was
    obtained.
  - Only CC0 or CC-BY material. A pack without its licence file and its
    SOURCE.txt must not be added.
  - Files that are not models (licence texts, SOURCE.txt, images, .bin) are
    copied with their pack but never become cases.

NOTHING HERE IS EVER PACKED INTO A NUGET PACKAGE: this is test material of a
test-side library that is never packed.
