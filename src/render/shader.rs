use std::{
    fs, io,
    ops::Range,
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Clone, Debug)]
pub struct ShaderInclude {
    /// The path of the included file.
    pub path: PathBuf,
    /// The index of the include directive in the source code.
    pub range: Range<usize>,
}

#[derive(Clone, Debug)]
pub struct ShaderFile {
    pub pragma_once: bool,
    pub source: String,
    pub path: PathBuf,
    pub includes: Vec<ShaderInclude>,
}

impl ShaderFile {
    pub const PRAGMA_ONCE: &'static str = "#pragma once";
    pub const INCLUDE: &'static str = "#include";

    fn strip_pragma_once(source: String) -> (String, bool) {
        match source.strip_prefix(Self::PRAGMA_ONCE) {
            Some(source) => (source.to_string(), true),
            None => (source, false),
        }
    }

    fn find_include_directives(source: &str) -> Result<Vec<ShaderInclude>, ShaderError> {
        let mut includes = Vec::new();

        let mut offset = 0;

        while let Some(start) = source[offset..].find(Self::INCLUDE) {
            let include_end = start + Self::INCLUDE.len();
            let source = &source[include_end + offset..];

            let Some(path_start) = source.find('"') else {
                return Err(ShaderError::ExpectedPathAfterIncludeDirective);
            };

            let path = &source[path_start + 1..];
            let Some(path_end) = path.find('"') else {
                return Err(ShaderError::ExpectedPathAfterIncludeDirective);
            };

            let path = PathBuf::from(&path[..path_end]);

            let end = include_end + path_end + 3;
            let range = start + offset..end + offset;

            includes.push(ShaderInclude { path, range });

            offset += end;
        }

        Ok(includes)
    }

    pub fn open(path: &Path) -> Result<Self, ShaderError> {
        let source = fs::read_to_string(&path)?;
        let (source, pragma_once) = Self::strip_pragma_once(source);
        let includes = Self::find_include_directives(&source)?;

        Ok(Self {
            pragma_once,
            source,
            path: path.to_path_buf(),
            includes,
        })
    }

    pub fn parent(&self) -> Result<&Path, ShaderError> {
        if let Some(parent) = self.path.parent() {
            Ok(parent)
        } else {
            Err(ShaderError::FileNotFound(self.path.clone()))
        }
    }
}

#[derive(Clone, Debug)]
pub struct ShaderProcessor {
    pub files: Vec<ShaderFile>,
}

impl ShaderProcessor {
    pub const fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn contains_shader(&self, path: &Path) -> bool {
        self.files.iter().any(|file| file.path == *path)
    }

    pub fn get_shader(&self, path: &Path) -> Option<&ShaderFile> {
        self.files.iter().find(|file| file.path == *path)
    }

    pub fn open_shader(&mut self, path: &Path) -> Result<&ShaderFile, ShaderError> {
        if self.contains_shader(path) {
            return Ok(self.get_shader(path).unwrap());
        }

        let shader = ShaderFile::open(path)?;
        self.files.push(shader);
        Ok(self.get_shader(&path).unwrap())
    }

    fn process_shader_recursive(
        &mut self,
        shader: &ShaderFile,
        included: &mut Vec<PathBuf>,
    ) -> Result<String, ShaderError> {
        let mut source = shader.source.clone();

        for include in shader.includes.iter().rev() {
            // open the shader file
            let include_path = shader.parent()?.join(&include.path);
            self.open_shader(&include_path)?;

            let include_shader = self.get_shader(&include_path).unwrap().clone();

            // check if the shader has already been included
            if included.contains(&include_path) && include_shader.pragma_once {
                source.replace_range(include.range.clone(), "");

                continue;
            }

            // mark the shader as included
            included.push(include_path.clone());

            // process the shader
            let include_source = self.process_shader_recursive(&include_shader, included)?;

            // replace the include directive with the source code
            source.replace_range(include.range.clone(), &include_source);
        }

        Ok(source)
    }

    pub fn process_shader(&mut self, path: impl AsRef<Path>) -> Result<String, ShaderError> {
        self.open_shader(path.as_ref())?;
        let shader = self.get_shader(path.as_ref()).unwrap().clone();

        let mut included = Vec::new();
        included.push(shader.path.clone());

        self.process_shader_recursive(&shader, &mut included)
    }
}

pub fn open_shader(
    device: &wgpu::Device,
    path: impl AsRef<Path>,
) -> Result<wgpu::ShaderModule, ShaderError> {
    static GLOBAL_PROCESSOR: Mutex<ShaderProcessor> = Mutex::new(ShaderProcessor::new());
    let source = GLOBAL_PROCESSOR.lock().unwrap().process_shader(&path)?;

    Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("Oakum shader: {}", path.as_ref().display())),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    }))
}

#[derive(Debug, thiserror::Error)]
pub enum ShaderError {
    #[error("Shader file not found: {0}")]
    FileNotFound(PathBuf),
    #[error("Expected path after #include directive")]
    ExpectedPathAfterIncludeDirective,
    #[error("Shader file not found: {0}")]
    IoError(#[from] io::Error),
}
