Model data fixtures (the CodeBrix model data format, ".cbmd") for the Rust reader tests of this crate.
These files are written by the .NET ModelDataWriter (src/CodeBrix.WebPlatform.WebGpu.Graphics/Models) and are NEVER edited by hand.
  triangle.cbmd                   one model "triangle": one triangle, one material "red", no texture (the golden model).
  every-section.cbmd              one model "every-section" that fills every section: a 2 x 2 texture and a sampler, two meshes,
                                  a node hierarchy with a fixed matrix, a skin, a morph target, animations with every
                                  interpolation, warnings, approximated material features and 32-bit indices.
  two-models-shared-texture.cbmd  two models, "first" and "second", that name the same texture "shared", stored once.
The test class ModelDataFixturesTests (tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests/Models) builds the same sets in code and
checks that these files are exactly what the writer writes; ModelDataFixtures.cs beside it describes every value they hold.
To regenerate them after a deliberate change, run that test with the environment variable CODEBRIX_REGENERATE_FIXTURES=1.
