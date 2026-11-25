//! THE PROOF AGAINST THE REFERENCE CONVERTER: every model file of the reference folder is loaded
//! by this loader and compared, value by value, with the model data the C# reference converter
//! (tests/CodeBrix.WebPlatform.WebGpu.Graphics.Tests.GltfReference, the converter this loader is
//! a port of) made from the same file.
//!
//! The folder is NOT in the crate (it travels inside the package): the environment variable
//! CODEBRIX_GRAPHICS_TEST_GLTF_REFERENCE names it, and the .NET test that runs these tests writes
//! it first (`GltfReferenceFolder.Write`: `cases.txt`, `models/`, `blobs/`). Without the variable
//! the test prints a SKIPPED line and passes.
//!
//! WHAT "THE SAME" MEANS HERE. EXACT, bit for bit: every count, index, name, flag and enumeration;
//! the index buffer; every vertex (positions, normals and tangents - read or generated -, texture
//! coordinates, colours), skin joints and weights, bounds, morph deltas, node transforms and
//! matrices (also those decomposed from a matrix), inverse bind matrices, key times and key
//! values, material factors, texture transforms, samplers; the pixels of PNG textures; the
//! reference's warnings, word for word. The classes of `TOLERANCES` exist so that the largest
//! difference of each kind is printed with every run; all but the JPEG ones are ZERO, because the
//! loader does the reference's arithmetic operation for operation (see `numerics.rs`) and no
//! difference was ever seen. JPEG pixels come from two different decoders and differ, most where
//! colour changes sharply in a picture whose colour planes are stored at half resolution: the
//! bound there is stated beside what was seen.
//!
//! NOT COMPARED, each intended and noted in the output: the pixels of a texture reduced by
//! `max_texture_size` (the two resample differently; the sizes are compared); the texture and
//! sampler LISTS of a model (the reference also decodes textures of extensions no slot uses;
//! what each slot points at is compared instead); warnings only the loader gives.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use crate::gltf::textures::ImageFormat;
use crate::gltf::{GltfDocument, GltfLoadOptions, GltfModelSet};
use crate::models::{ModelData, ModelDataSet, ModelMaterial, ModelSkinVertex, ModelTextureSlot, ModelVertex};

/// The tolerance of each class of value (absolute difference), and what the class is. Zero means
/// bit for bit. The JPEG bounds are in 1/255 of a channel; seen on the Khronos sample models: a
/// largest difference of 23 with a mean of 1.57 on a normal map stored with half-resolution
/// colour planes, and 1 with a mean of 0.006 on a full-resolution picture.
const TOLERANCES: [(&str, f32, &str); 9] = [
    ("exact", 0.0, "read from the file, or computed from it by plain arithmetic"),
    ("normal", 0.0, "normals: read from the file, or generated"),
    ("tangent", 0.0, "tangents: read from the file, or generated"),
    ("skin weight", 0.0, "skin weights divided by their sum"),
    ("rotation", 0.0, "unit quaternions: node rotations and rotation keys after normalisation"),
    ("matrix", 0.0, "node matrices built from translation, rotation and scale"),
    ("texture transform", 0.0, "KHR_texture_transform matrices"),
    ("jpeg pixel (largest)", 32.0, "JPEG pixels, the largest difference of one channel of one pixel, in 1/255: two different decoders"),
    ("jpeg pixel (mean)", 2.5, "JPEG pixels, the mean difference of a channel, in 1/255: two different decoders"),
];

/// One line of `cases.txt`.
struct Case {
    name: String,
    models: Vec<String>,
    options: GltfLoadOptions,
    max_size: u32,
    /// The blob file, or the reference converter's refusal message.
    blob: Result<String, String>,
}

/// What a comparison found.
#[derive(Default)]
struct Report {
    /// Differences that are not allowed.
    differences: Vec<String>,
    /// The largest difference seen per tolerance class.
    largest: BTreeMap<&'static str, f32>,
    /// Things worth a line that are not differences.
    notes: Vec<String>,
}

impl Report {
    fn differs(&mut self, what: String) {
        if self.differences.len() < 12 {
            self.differences.push(what);
        } else if self.differences.len() == 12 {
            self.differences.push("... (more differences)".to_string());
        }
    }

    fn same<T: PartialEq + std::fmt::Debug>(&mut self, what: &dyn Fn() -> String, reference: &T, ours: &T) {
        if reference != ours {
            self.differs(format!("{}: reference {reference:?}, loader {ours:?}", what()));
        }
    }

    fn close(&mut self, class: &'static str, what: &dyn Fn() -> String, reference: f32, ours: f32) {
        if reference.to_bits() == ours.to_bits() || (reference == 0.0 && ours == 0.0) {
            return;
        }
        let difference = (reference - ours).abs();
        let tolerance = TOLERANCES.iter().find(|(name, _, _)| *name == class).map_or(0.0, |(_, tolerance, _)| *tolerance);
        let largest = self.largest.entry(class).or_insert(0.0);
        if difference.is_nan() || difference > *largest {
            *largest = if difference.is_nan() { f32::INFINITY } else { difference };
        }
        if difference.is_nan() || difference > tolerance {
            self.differs(format!("{} [{class}]: reference {reference:e}, loader {ours:e} (apart by {difference:e})", what()));
        }
    }

    fn close_all(&mut self, class: &'static str, what: &dyn Fn() -> String, reference: &[f32], ours: &[f32]) {
        if reference.len() != ours.len() {
            self.differs(format!("{}: reference has {} values, loader {}", what(), reference.len(), ours.len()));
            return;
        }
        for (index, (a, b)) in reference.iter().zip(ours).enumerate() {
            self.close(class, &|| format!("{} [{index}]", what()), *a, *b);
        }
    }
}

fn reference_folder() -> Option<PathBuf> {
    match std::env::var("CODEBRIX_GRAPHICS_TEST_GLTF_REFERENCE") {
        Ok(folder) => Some(PathBuf::from(folder)),
        Err(_) => {
            eprintln!("SKIPPED: no glTF reference folder (set CODEBRIX_GRAPHICS_TEST_GLTF_REFERENCE to the folder GltfReferenceFolder.Write wrote)");
            None
        }
    }
}

fn read_cases(folder: &Path) -> Vec<Case> {
    let path = folder.join("cases.txt");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("CODEBRIX_GRAPHICS_TEST_GLTF_REFERENCE is set, but {} cannot be read: {e}", path.display()));
    let mut cases = Vec::new();
    for line in text.lines().filter(|line| !line.is_empty() && !line.starts_with('#')) {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 4, "a line of cases.txt has four tab-separated fields: {line}");
        let mut options = GltfLoadOptions { share_vertex_data: false, ..GltfLoadOptions::default() };
        let mut max_size = 0;
        for option in fields[2].split(',') {
            let (key, value) = option.split_once('=').unwrap_or_else(|| panic!("an option of cases.txt is key=value: {option}"));
            match key {
                "textures" => options.load_textures = value == "true",
                "mipmaps" => options.mipmaps = value == "true",
                "srgb" => options.color_textures_srgb = value == "true",
                "tangents" => options.generate_tangents = value == "true",
                "maxsize" => max_size = value.parse().expect("maxsize is a number"),
                other => panic!("cases.txt has the unknown option {other}"),
            }
        }
        options.max_texture_size = max_size;
        let blob = match fields[3].strip_prefix('!') {
            Some(message) => Err(message.to_string()),
            None => Ok(fields[3].to_string()),
        };
        cases.push(Case { name: fields[0].to_string(), models: fields[1].split('|').map(str::to_string).collect(), options, max_size, blob });
    }
    cases
}

/// Loads the models of a case with this loader, as an application would: parse, read the files
/// the document names, build.
fn load_case(folder: &Path, case: &Case) -> Result<GltfModelSet, String> {
    let models = folder.join("models");
    let mut set = GltfModelSet::new();
    for model in &case.models {
        let path = models.join(model);
        let bytes = std::fs::read(&path).map_err(|e| format!("{} cannot be read: {e}", path.display()))?;
        let name = Path::new(model).file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let base = model.rsplit_once('/').map_or("", |(folder, _)| folder).to_string();
        let document = GltfDocument::parse(&bytes).map_err(|e| e.message().to_string())?;
        let mut files: HashMap<String, Vec<u8>> = HashMap::new();
        for file in document.external_files(Some(&base), case.options.load_textures) {
            if let Ok(bytes) = std::fs::read(models.join(&file.address)) {
                files.insert(file.address, bytes);
            }
        }
        set.add(&document, &name, Some(&base), &|address| files.get(address).map(Vec::as_slice), &case.options).map_err(|e| e.message().to_string())?;
    }
    Ok(set)
}

/// One side of a comparison: a set, the model looked at, and (for the loader's side) the format
/// each texture's image file had.
#[derive(Clone, Copy)]
struct Side<'a> {
    set: &'a ModelDataSet<'a>,
    model: &'a ModelData<'a>,
    formats: &'a [ImageFormat],
}

fn vertices(model: &ModelData<'_>) -> Vec<ModelVertex> {
    model.vertex_bytes.chunks_exact(ModelVertex::SIZE).map(bytemuck::pod_read_unaligned).collect()
}

fn skin_vertices(model: &ModelData<'_>) -> Vec<ModelSkinVertex> {
    model.skin_bytes.chunks_exact(ModelSkinVertex::SIZE).map(bytemuck::pod_read_unaligned).collect()
}

/// Compares one texture slot: where it points (the texture's content, the sampler's content),
/// not the indices themselves - the reference keeps textures in its lists that no slot uses.
fn compare_slot(report: &mut Report, what: &dyn Fn() -> String, reference: &ModelTextureSlot, ours: &ModelTextureSlot, reference_set: Side<'_>, our_set: Side<'_>, case: &Case) {
    report.same(&|| format!("{} texture index", what()), &reference.texture_index, &ours.texture_index);
    report.same(&|| format!("{} texture coordinate set", what()), &reference.tex_coord, &ours.tex_coord);
    report.same(&|| format!("{} has transform", what()), &reference.has_transform, &ours.has_transform);
    report.close_all("texture transform", &|| format!("{} transform", what()), &reference.transform, &ours.transform);
    let texture = |side: Side<'_>, slot: &ModelTextureSlot| slot.texture.map(|local| side.model.textures[local]);
    match (texture(reference_set, reference), texture(our_set, ours)) {
        (None, None) => {}
        (Some(a), Some(b)) => {
            let format = our_set.formats.get(b).copied().unwrap_or(ImageFormat::Other);
            let (a, b) = (&reference_set.set.textures[a], &our_set.set.textures[b]);
            report.same(&|| format!("{} texture name", what()), &a.name, &b.name);
            report.same(&|| format!("{} texture size", what()), &(a.width, a.height), &(b.width, b.height));
            report.same(&|| format!("{} texture colour space and mipmaps", what()), &(a.srgb, a.mipmaps), &(b.srgb, b.mipmaps));
            if case.max_size == 0 && a.pixels.len() == b.pixels.len() && a.pixels != b.pixels {
                // Only JPEG pixels may differ: the two decoders are different programs (they
                // differ most where colour changes sharply, in how they spread the half-resolution
                // colour planes). PNG pixels are the same bytes.
                let largest = a.pixels.iter().zip(b.pixels.iter()).map(|(x, y)| x.abs_diff(*y)).max().unwrap_or(0);
                let mean = a.pixels.iter().zip(b.pixels.iter()).map(|(x, y)| x.abs_diff(*y) as f64).sum::<f64>() / a.pixels.len() as f64;
                if format == ImageFormat::Jpeg {
                    report.close("jpeg pixel (largest)", &|| format!("{} pixels, largest difference of a channel in 1/255", what()), 0.0, largest as f32);
                    report.close("jpeg pixel (mean)", &|| format!("{} pixels, mean difference of a channel in 1/255", what()), 0.0, mean as f32);
                } else {
                    report.differs(format!("{} pixels of a PNG image differ (largest difference of a channel {largest})", what()));
                }
            }
        }
        (a, b) => report.differs(format!("{} texture: reference {a:?}, loader {b:?}", what())),
    }
    let sampler = |side: Side<'_>, slot: &ModelTextureSlot| slot.sampler.map(|local| side.model.samplers[local].clone());
    report.same(&|| format!("{} sampler", what()), &sampler(reference_set, reference), &sampler(our_set, ours));
}

fn compare_material(report: &mut Report, what: &dyn Fn() -> String, reference: &ModelMaterial, ours: &ModelMaterial, reference_set: Side<'_>, our_set: Side<'_>, case: &Case) {
    report.same(&|| format!("{} index and name", what()), &(reference.index, &reference.name), &(ours.index, &ours.name));
    report.close_all("exact", &|| format!("{} base colour", what()), &reference.base_color_factor.to_array(), &ours.base_color_factor.to_array());
    report.close_all("exact", &|| format!("{} emissive", what()), &reference.emissive_factor.to_array(), &ours.emissive_factor.to_array());
    report.close_all("exact", &|| format!("{} specular colour", what()), &reference.specular_color.to_array(), &ours.specular_color.to_array());
    let scalars = |m: &ModelMaterial| [m.metallic_factor, m.roughness_factor, m.normal_scale, m.occlusion_strength, m.emissive_strength, m.alpha_cutoff, m.index_of_refraction, m.clear_coat_factor, m.clear_coat_roughness, m.specular_factor, m.transmission_factor];
    report.close_all("exact", &|| format!("{} factors (metallic, roughness, normal scale, occlusion, emissive strength, cutoff, ior, clearcoat, clearcoat roughness, specular, transmission)", what()), &scalars(reference), &scalars(ours));
    report.same(&|| format!("{} alpha mode, double sided, unlit", what()), &(reference.alpha_mode, reference.double_sided, reference.unlit), &(ours.alpha_mode, ours.double_sided, ours.unlit));
    report.same(&|| format!("{} approximated features", what()), &reference.approximated_features, &ours.approximated_features);
    let names = ["base colour", "metallic-roughness", "normal", "occlusion", "emissive"];
    for (index, (a, b)) in reference.slots().into_iter().zip(ours.slots()).enumerate() {
        compare_slot(report, &|| format!("{} {} slot", what(), names[index]), a, b, reference_set, our_set, case);
    }
}

/// Compares one model. The allowed differences, each intended: the reference's texture and
/// sampler LISTS may hold entries no slot uses (it decodes the textures of extensions it only
/// notes), so textures and samplers are compared where slots point; warnings are compared as the
/// reference's texts (the loader has warnings of its own for what it checks and the reference
/// does not).
fn compare_models(report: &mut Report, reference_set: &ModelDataSet<'_>, our_set: &ModelDataSet<'_>, formats: &[ImageFormat], index: usize, case: &Case) {
    let (reference, ours) = (&reference_set.models[index], &our_set.models[index]);
    let (reference_side, our_side) = (Side { set: reference_set, model: reference, formats: &[] }, Side { set: our_set, model: ours, formats });
    let name = reference.name.clone();
    let what = |part: &str| format!("model '{name}' {part}");
    report.same(&|| what("name"), &reference.name, &ours.name);
    report.same(&|| what("vertex count"), &reference.vertex_count, &ours.vertex_count);
    report.same(&|| what("index count and format"), &(reference.index_count, reference.index_format), &(ours.index_count, ours.index_format));
    report.same(&|| what("skin vertex count"), &reference.skin_vertex_count, &ours.skin_vertex_count);
    if reference.index_bytes != ours.index_bytes {
        report.differs(what("index bytes differ"));
    }
    let (reference_vertices, our_vertices) = (vertices(reference), vertices(ours));
    for (vertex, (a, b)) in reference_vertices.iter().zip(&our_vertices).enumerate() {
        report.close_all("exact", &|| what(&format!("vertex {vertex} position")), &a.position.to_array(), &b.position.to_array());
        report.close_all("normal", &|| what(&format!("vertex {vertex} normal")), &a.normal.to_array(), &b.normal.to_array());
        report.close_all("tangent", &|| what(&format!("vertex {vertex} tangent")), &a.tangent, &b.tangent);
        report.close_all("exact", &|| what(&format!("vertex {vertex} texture coordinates")), &[a.tex_coord0.x, a.tex_coord0.y, a.tex_coord1.x, a.tex_coord1.y], &[b.tex_coord0.x, b.tex_coord0.y, b.tex_coord1.x, b.tex_coord1.y]);
        report.same(&|| what(&format!("vertex {vertex} colour")), &a.color, &b.color);
    }
    for (vertex, (a, b)) in skin_vertices(reference).iter().zip(&skin_vertices(ours)).enumerate() {
        report.same(&|| what(&format!("skin vertex {vertex} joints")), &a.joints, &b.joints);
        report.close_all("skin weight", &|| what(&format!("skin vertex {vertex} weights")), &a.weights, &b.weights);
    }

    report.same(&|| what("mesh count"), &reference.meshes.len(), &ours.meshes.len());
    for (mesh, (a, b)) in reference.meshes.iter().zip(&ours.meshes).enumerate() {
        report.same(&|| what(&format!("mesh {mesh} name")), &a.name, &b.name);
        report.close_all("exact", &|| what(&format!("mesh {mesh} default morph weights")), &a.default_morph_weights, &b.default_morph_weights);
        report.same(&|| what(&format!("mesh {mesh} primitive count")), &a.primitives.len(), &b.primitives.len());
        for (primitive, (p, q)) in a.primitives.iter().zip(&b.primitives).enumerate() {
            let place = format!("mesh {mesh} primitive {primitive}");
            report.same(&|| what(&format!("{place} ranges (index, material, first index, index count, base vertex, vertex count, skin base)")), &(p.index, p.material, p.first_index, p.index_count, p.base_vertex, p.vertex_count, p.skin_base_vertex), &(q.index, q.material, q.first_index, q.index_count, q.base_vertex, q.vertex_count, q.skin_base_vertex));
            report.same(&|| what(&format!("{place} flags (colours, second texture coordinates, tangents, tangents generated, normals generated)")), &(p.has_vertex_colors, p.has_tex_coord1, p.has_tangents, p.tangents_generated, p.normals_generated), &(q.has_vertex_colors, q.has_tex_coord1, q.has_tangents, q.tangents_generated, q.normals_generated));
            report.close_all("exact", &|| what(&format!("{place} bounds")), &[p.bounds_min.to_array(), p.bounds_max.to_array()].concat(), &[q.bounds_min.to_array(), q.bounds_max.to_array()].concat());
            report.same(&|| what(&format!("{place} morph target count")), &p.morph_targets.len(), &q.morph_targets.len());
            for (target, (s, t)) in p.morph_targets.iter().zip(&q.morph_targets).enumerate() {
                let flat = |deltas: &[glam::Vec3]| deltas.iter().flat_map(|d| d.to_array()).collect::<Vec<f32>>();
                report.close_all("exact", &|| what(&format!("{place} morph target {target} position deltas")), &flat(&s.position_deltas), &flat(&t.position_deltas));
                report.close_all("exact", &|| what(&format!("{place} morph target {target} normal deltas")), &flat(&s.normal_deltas), &flat(&t.normal_deltas));
                report.close_all("exact", &|| what(&format!("{place} morph target {target} tangent deltas")), &flat(&s.tangent_deltas), &flat(&t.tangent_deltas));
            }
        }
    }

    report.same(&|| what("material count"), &reference.materials.len(), &ours.materials.len());
    for (material, (a, b)) in reference.materials.iter().zip(&ours.materials).enumerate() {
        compare_material(report, &|| what(&format!("material {material}")), a, b, reference_side, our_side, case);
    }
    compare_material(report, &|| what("default material"), &reference.default_material, &ours.default_material, reference_side, our_side, case);
    if reference.textures.len() != ours.textures.len() || reference.samplers.len() != ours.samplers.len() {
        report.notes.push(format!("model '{name}': the reference lists {} texture(s) and {} sampler(s), the loader {} and {} (the reference also decodes textures no slot uses)", reference.textures.len(), reference.samplers.len(), ours.textures.len(), ours.samplers.len()));
    }

    report.same(&|| what("node count"), &reference.nodes.len(), &ours.nodes.len());
    for (node, (a, b)) in reference.nodes.iter().zip(&ours.nodes).enumerate() {
        report.same(&|| what(&format!("node {node} (index, name, parent, children, mesh, skin, fixed matrix, in scene)")), &(a.index, &a.name, a.parent, &a.children, a.mesh, a.skin, a.has_fixed_matrix, a.in_scene), &(b.index, &b.name, b.parent, &b.children, b.mesh, b.skin, b.has_fixed_matrix, b.in_scene));
        let (s, t) = (&a.default_transform, &b.default_transform);
        report.close_all("exact", &|| what(&format!("node {node} translation and scale")), &[s.translation.to_array(), s.scale.to_array()].concat(), &[t.translation.to_array(), t.scale.to_array()].concat());
        report.close_all("rotation", &|| what(&format!("node {node} rotation")), &s.rotation.to_array(), &t.rotation.to_array());
        report.close_all("matrix", &|| what(&format!("node {node} local matrix")), &a.local_matrix.to_cols_array(), &b.local_matrix.to_cols_array());
        report.close_all("exact", &|| what(&format!("node {node} default morph weights")), &a.default_morph_weights, &b.default_morph_weights);
    }
    report.same(&|| what("root nodes"), &reference.root_nodes, &ours.root_nodes);

    report.same(&|| what("skin count"), &reference.skins.len(), &ours.skins.len());
    for (skin, (a, b)) in reference.skins.iter().zip(&ours.skins).enumerate() {
        report.same(&|| what(&format!("skin {skin} (index, name, joints, skeleton)")), &(a.index, &a.name, &a.joints, a.skeleton), &(b.index, &b.name, &b.joints, b.skeleton));
        let flat = |matrices: &[glam::Mat4]| matrices.iter().flat_map(|m| m.to_cols_array()).collect::<Vec<f32>>();
        report.close_all("exact", &|| what(&format!("skin {skin} inverse bind matrices")), &flat(&a.inverse_bind_matrices), &flat(&b.inverse_bind_matrices));
    }

    report.same(&|| what("animation count"), &reference.animations.len(), &ours.animations.len());
    for (animation, (a, b)) in reference.animations.iter().zip(&ours.animations).enumerate() {
        report.same(&|| what(&format!("animation {animation} name and channel count")), &(&a.name, a.channels.len()), &(&b.name, b.channels.len()));
        for (channel, (c, d)) in a.channels.iter().zip(&b.channels).enumerate() {
            let place = format!("animation {animation} channel {channel}");
            report.same(&|| what(&format!("{place} (node, path, interpolation, components)")), &(c.node, c.path, c.interpolation, c.component_count), &(d.node, d.path, d.interpolation, d.component_count));
            report.close_all("exact", &|| what(&format!("{place} times")), &c.times, &d.times);
            let class = if c.path == crate::AnimationPath::Rotation { "rotation" } else { "exact" };
            report.close_all(class, &|| what(&format!("{place} values")), &c.values, &d.values);
        }
    }

    // The reference's warnings must all be the loader's too, in order, word for word - except
    // the ones about its own reading (its glTF library's validation, its imaging library).
    let theirs: Vec<&String> = reference.warnings.iter().filter(|w| !w.starts_with("The file failed glTF validation")).collect();
    let mut position = 0;
    for warning in &theirs {
        match ours.warnings[position..].iter().position(|w| w == *warning) {
            Some(found) => position += found + 1,
            None => report.differs(what(&format!("warning of the reference that the loader does not give (in this order): {warning}"))),
        }
    }
    for warning in reference.warnings.iter().filter(|w| w.starts_with("The file failed glTF validation")) {
        report.notes.push(format!("model '{name}': the reference read the file without validation: {warning}"));
    }
    for warning in ours.warnings.iter().filter(|w| !theirs.contains(w)) {
        report.notes.push(format!("model '{name}': a warning of the loader alone: {warning}"));
    }
}

#[test]
fn every_reference_model_loads_to_the_reference_converter_s_model_data() {
    let Some(folder) = reference_folder() else {
        return;
    };
    let cases = read_cases(&folder);
    assert!(!cases.is_empty(), "cases.txt lists no case");
    let mut failed = Vec::new();
    let mut largest: BTreeMap<&'static str, f32> = BTreeMap::new();
    let (mut compared, mut refused_by_both, mut refused_by_reference_only) = (0, 0, 0);
    for case in &cases {
        let loaded = load_case(&folder, case);
        let blob_name = match (&case.blob, &loaded) {
            (Err(message), Err(ours)) => {
                refused_by_both += 1;
                println!("{}: refused by both (reference: {message} | loader: {ours})", case.name);
                // The loader's refusal of a compressed model names the extension.
                for extension in ["KHR_draco_mesh_compression", "EXT_meshopt_compression"] {
                    if message.contains(extension) && !ours.contains(extension) {
                        failed.push(format!("{}: the loader's refusal does not name {extension}: {ours}", case.name));
                    }
                }
                continue;
            }
            (Err(message), Ok(_)) => {
                // The reference's glTF library refuses some files this loader reads (an empty
                // JSON array, an extension it does not list); it must never be the other way
                // round for the compression extensions, which the loader refuses by name.
                refused_by_reference_only += 1;
                println!("{}: refused by the reference only: {message}", case.name);
                if message.contains("draco") || message.contains("Draco") || message.contains("meshopt") {
                    failed.push(format!("{}: the reference refused a compressed model the loader accepted", case.name));
                }
                continue;
            }
            (Ok(_), Err(ours)) => {
                failed.push(format!("{}: the loader refused a file the reference converted: {ours}", case.name));
                continue;
            }
            (Ok(blob), Ok(_)) => blob,
        };
        let ours = loaded.expect("checked above");
        let blob = std::fs::read(folder.join("blobs").join(blob_name)).unwrap_or_else(|e| panic!("the blob {blob_name} of case {} cannot be read: {e}", case.name));
        let reference = ModelDataSet::read(&blob).unwrap_or_else(|e| panic!("the reference blob of case {} is not model data: {}", case.name, e.message()));
        let our_set = ours.data_set();
        let formats: Vec<ImageFormat> = ours.textures.iter().map(|t| t.format).collect();
        let mut report = Report::default();
        report.same(&|| "model count".to_string(), &reference.models.len(), &our_set.models.len());
        for index in 0..reference.models.len().min(our_set.models.len()) {
            compare_models(&mut report, &reference, &our_set, &formats, index, case);
        }
        // Sharing between the models of a set: the same number of textures, unless the
        // reference kept textures no slot uses.
        if reference.textures.len() != our_set.textures.len() {
            report.notes.push(format!("the reference set holds {} texture(s), the loader's {}", reference.textures.len(), our_set.textures.len()));
        }
        compared += 1;
        for (class, value) in &report.largest {
            let entry = largest.entry(class).or_insert(0.0);
            *entry = entry.max(*value);
        }
        for note in &report.notes {
            println!("{}: note: {note}", case.name);
        }
        if report.differences.is_empty() {
            println!("{}: the same", case.name);
        } else {
            for difference in &report.differences {
                println!("{}: DIFFERENT: {difference}", case.name);
            }
            failed.push(format!("{}: {} difference(s), the first: {}", case.name, report.differences.len(), report.differences[0]));
        }
    }
    println!("{compared} case(s) compared, {refused_by_both} refused by both, {refused_by_reference_only} refused by the reference only");
    for (class, tolerance, why) in TOLERANCES {
        println!("class '{class}': tolerance {tolerance:e}, largest difference seen {:e} ({why})", largest.get(class).copied().unwrap_or(0.0));
    }
    assert!(failed.is_empty(), "{} of {} case(s) differ from the reference converter:\n{}", failed.len(), cases.len(), failed.join("\n"));
}

#[test]
fn the_comparison_finds_nothing_in_equal_sets_and_notices_one_changed_value() {
    use crate::gltf::test_support::{build_with, triangle_document};
    let options = GltfLoadOptions { share_vertex_data: false, ..GltfLoadOptions::default() };
    let case = Case { name: "self".to_string(), models: Vec::new(), options: options.clone(), max_size: 0, blob: Ok(String::new()) };
    let original = build_with(&triangle_document("", ""), &options);
    let same = build_with(&triangle_document("", ""), &options);
    let mut report = Report::default();
    compare_models(&mut report, &original.data_set(), &same.data_set(), &[], 0, &case);
    assert!(report.differences.is_empty(), "{:?}", report.differences);

    // One position moved by the smallest step a float has; one index changed; one name changed.
    let mut moved = same.clone();
    moved.models[0].vertices[1].position.x = f32::from_bits(moved.models[0].vertices[1].position.x.to_bits() + 1);
    let mut report = Report::default();
    compare_models(&mut report, &original.data_set(), &moved.data_set(), &[], 0, &case);
    assert_eq!(report.differences.len(), 1, "{:?}", report.differences);
    assert!(report.differences[0].contains("vertex 1 position") && report.differences[0].contains("[exact]"));

    let mut reindexed = same.clone();
    reindexed.models[0].index_bytes[0] = 1;
    let mut report = Report::default();
    compare_models(&mut report, &original.data_set(), &reindexed.data_set(), &[], 0, &case);
    assert!(report.differences.iter().any(|d| d.contains("index bytes differ")));

    let mut renamed = same.clone();
    renamed.models[0].nodes[0].name = Some("other".to_string());
    renamed.models[0].warnings.push("a warning of the loader alone".to_string());
    let mut report = Report::default();
    compare_models(&mut report, &original.data_set(), &renamed.data_set(), &[], 0, &case);
    assert_eq!(report.differences.len(), 1, "{:?}", report.differences);
    assert!(report.differences[0].contains("node 0") && report.notes.iter().any(|n| n.contains("a warning of the loader alone")));

    // A warning of the reference that the loader does not give is a difference.
    let mut warned = original.clone();
    warned.models[0].warnings.push("Mesh 0 primitive 1 draws POINTS; only triangles are supported, so it is left out.".to_string());
    let mut report = Report::default();
    compare_models(&mut report, &warned.data_set(), &same.data_set(), &[], 0, &case);
    assert!(report.differences.iter().any(|d| d.contains("draws POINTS")));
}

/// Every model file of the reference folder, cut short at several lengths and with bytes of its
/// document changed, must be answered with a model or a message - never a panic. (The test fails
/// only by panicking.) Textures are left out: the image decoders have tests of their own.
#[test]
fn no_reference_model_cut_short_or_with_changed_bytes_makes_the_loader_panic() {
    let Some(folder) = reference_folder() else {
        return;
    };
    let models = folder.join("models");
    let mut seen = std::collections::BTreeSet::new();
    let (mut tried, mut read) = (0u32, 0u32);
    for case in read_cases(&folder) {
        for model in case.models {
            if !seen.insert(model.clone()) {
                continue;
            }
            let Ok(bytes) = std::fs::read(models.join(&model)) else {
                continue;
            };
            let base = model.rsplit_once('/').map_or("", |(folder, _)| folder).to_string();
            let options = GltfLoadOptions { load_textures: false, ..GltfLoadOptions::default() };
            let mut attempt = |changed: &[u8]| {
                tried += 1;
                let Ok(document) = GltfDocument::parse(changed) else {
                    return;
                };
                let mut files: HashMap<String, Vec<u8>> = HashMap::new();
                for file in document.external_files(Some(&base), false) {
                    if let Ok(bytes) = std::fs::read(models.join(&file.address)) {
                        files.insert(file.address, bytes);
                    }
                }
                if GltfModelSet::new().add(&document, "changed", Some(&base), &|address| files.get(address).map(Vec::as_slice), &options).is_ok() {
                    read += 1;
                }
            };
            for cut in [0, 1, 4, 11, 12, 19, 20, 21, 64, bytes.len() / 3, bytes.len() / 2, bytes.len().saturating_sub(1)] {
                attempt(&bytes[..cut.min(bytes.len())]);
            }
            // The document is at the start of the file (the whole of a .gltf, the first chunk of
            // a .glb): change one byte at a time there, at positions and to values that a small
            // generator picks the same way on every run.
            let span = bytes.len().min(6000);
            let mut state = 0x2545_F491u32 ^ bytes.len() as u32;
            for _ in 0..40 {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let at = (state >> 8) as usize % span;
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let mut changed = bytes.clone();
                changed[at] = match (state >> 16) % 4 {
                    0 => b'9',
                    1 => b'0',
                    2 => b'-',
                    _ => (state >> 24) as u8,
                };
                attempt(&changed);
            }
        }
    }
    println!("{tried} damaged file(s) tried, {read} of them still read as a model, none panicked");
}
