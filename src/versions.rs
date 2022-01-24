use dom::Element;
use regex::Regex;
use std::fmt;

#[derive(Debug)]
pub struct ActiveVersion {
    api: String, // gl, gles1, gles2
    profile: String, // core/compatibility (gl), blank (gles)
    extension_space: String, // gl, glcore, gles1, gles2
    number: String, // 1.0, 3.2, etc.
}

impl fmt::Display for ActiveVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.extension_space.as_str() {
            "gl" => write!(f, "OpenGL {}", self.number),
            "gles1" | "gles2" => write!(f, "OpenGL ES {}", self.number),
            "glcore" => write!(f, "OpenGL Core {}", self.number),
            x => write!(f, "{} {}", x, self.number),
        }
    }
}

impl ActiveVersion {
    pub fn correct_api(&self, el: &Element) -> bool {
        match el.get_attributes().get("api") {
            Some(api) => *api == self.api,
            _ => true,
        }
    }
    pub fn correct_version(&self, el: &Element) -> bool {
        match el.get_attributes().get("number") {
            Some(vers) => *vers <= self.number,
            _ => false,
        }
    }
    pub fn correct_profile(&self, el: &Element) -> bool {
        match el.get_attributes().get("profile") {
            Some(prof) => *prof == self.profile,
            _ => true,
        }
    }
    pub fn supported(&self, el: &Element) -> bool {
        match el.get_attributes().get("supported") {
            Some(supp) => {
                for sub in supp.split("|") {
                    if sub == self.api { return true }
                }
                false
            },
            _ => false,
        }
    }
}

pub fn parse_version(src: &str) -> Result<ActiveVersion,&str> {
    let (api, profile, extension_space, number);
    if src.starts_with("gles1") {
        api = "gles1";
        profile = "";
        extension_space = "gles1";
        number = &src[4..]; // include the 1 in the number
    }
    else if src.starts_with("gles") {
        api = "gles2";
        profile = "";
        extension_space = "gles2";
        number = &src[4..]; // include the 2 in the number
    }
    else if src.starts_with("glcore") {
        api = "gl";
        profile = "core";
        extension_space = "glcore";
        number = &src[6..];
    }
    else if src.starts_with("gl") {
        api = "gl";
        profile = "compatibility";
        extension_space = "gl";
        number = &src[2..];
    }
    else {
        return Err("must start with gl, glcore, or gles");
    }
    lazy_static! {
        static ref VALID_VERSION: Regex = Regex::new(r"^[0-9]\.[0-9]$")
            .unwrap();
    }
    if !VALID_VERSION.is_match(number) {
        return Err("must end with a valid version number (X.Y)");
    }
    Ok(ActiveVersion {
        api: api.to_owned(),
        profile: profile.to_owned(),
        extension_space: extension_space.to_owned(),
        number: number.to_owned()
    })
}

