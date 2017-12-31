use std::iter;
use std::io::{Read, Write};

use index_map::IndexMap;
use parity_wasm::elements::{Deserialize, Error, Serialize, VarUint32, VarUint7};

const NAME_TYPE_MODULE: u8 = 0;
const NAME_TYPE_FUNCTION: u8 = 1;
const NAME_TYPE_LOCAL: u8 = 2;

/// Debug name information.
#[derive(Clone, Debug, PartialEq)]
pub enum NameSection {
    /// Module name section.
    Module(ModuleNameSection),

    /// Function name section.
    Function(FunctionNameSection),

    /// Local name section.
    Local(LocalNameSection),

    /// Name section is unparsed.
    Unparsed {
        name_type: u8,
        name_payload: Vec<u8>,
    }
}

impl Serialize for NameSection {
    type Error = Error;

    fn serialize<W: Write>(self, wtr: &mut W) -> Result<(), Error> {
        let (name_type, name_payload) = match self {
            NameSection::Module(mod_name) => {
                let mut buffer = vec![];
                mod_name.serialize(&mut buffer)?;
                (NAME_TYPE_MODULE, buffer)
            }
            NameSection::Function(fn_names) => {
                let mut buffer = vec![];
                fn_names.serialize(&mut buffer)?;
                (NAME_TYPE_FUNCTION, buffer)
            }
            NameSection::Local(local_names) => {
                let mut buffer = vec![];
                local_names.serialize(&mut buffer)?;
                (NAME_TYPE_LOCAL, buffer)
            }
            NameSection::Unparsed { name_type, name_payload } => {
                (name_type, name_payload)
            }
        };
        VarUint7::from(name_type).serialize(wtr)?;
        VarUint32::from(name_payload.len()).serialize(wtr)?;
        wtr.write_all(&name_payload)?;
        Ok(())
    }
}

impl Deserialize for NameSection {
    type Error = Error;

    fn deserialize<R: Read>(rdr: &mut R) -> Result<NameSection, Error> {
        let name_type: u8 = VarUint7::deserialize(rdr)?.into();
        let name_payload_len: u32 = VarUint32::deserialize(rdr)?.into();
        let name_section = match name_type {
            NAME_TYPE_MODULE => {
                NameSection::Module(ModuleNameSection::deserialize(rdr)?)
            }
            NAME_TYPE_FUNCTION => {
                NameSection::Function(FunctionNameSection::deserialize(rdr)?)
            }
            NAME_TYPE_LOCAL => {
                NameSection::Local(LocalNameSection::deserialize(rdr)?)
            }
            _ => {
                let mut name_payload = vec![0u8; name_payload_len as usize];
                rdr.read_exact(&mut name_payload)?;
                NameSection::Unparsed { name_type, name_payload }
            }
        };
        Ok(name_section)
    }
}

/// The name of this module.
#[derive(Clone, Debug, PartialEq)]
pub struct ModuleNameSection {
    name: String,
}

impl ModuleNameSection {
    /// Create a new module name section with the specified name.
    pub fn new<S: Into<String>>(name: S) -> ModuleNameSection {
        ModuleNameSection { name: name.into() }
    }

    /// The name of this module.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name of this module (mutable).
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
}

impl Serialize for ModuleNameSection {
    type Error = Error;

    fn serialize<W: Write>(self, wtr: &mut W) -> Result<(), Error> {
        self.name.serialize(wtr)
    }
}

impl Deserialize for ModuleNameSection {
    type Error = Error;

    fn deserialize<R: Read>(rdr: &mut R) -> Result<ModuleNameSection, Error> {
        let name = String::deserialize(rdr)?;
        Ok(ModuleNameSection { name })
    }
}

/// The names of the functions in this module.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FunctionNameSection {
    names: NameMap,
}

impl FunctionNameSection {
    /// A map from function indices to names.
    pub fn names(&self) -> &NameMap {
        &self.names
    }

    /// A map from function indices to names (mutable).
    pub fn names_mut(&mut self) -> &mut NameMap {
        &mut self.names
    }
}

impl Serialize for FunctionNameSection {
    type Error = Error;

    fn serialize<W: Write>(self, wtr: &mut W) -> Result<(), Error> {
        self.names.serialize(wtr)
    }
}

impl Deserialize for FunctionNameSection {
    type Error = Error;

    fn deserialize<R: Read>(rdr: &mut R) -> Result<FunctionNameSection, Error> {
        let names = IndexMap::deserialize(rdr)?;
        Ok(FunctionNameSection { names })
    }
}

/// The names of the local variables in this module's functions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LocalNameSection {
    local_names: IndexMap<NameMap>,
}

impl LocalNameSection {
    /// A map from function indices to a map from variables indices to names.
    pub fn local_names(&self) -> &IndexMap<NameMap> {
        &self.local_names
    }

    /// A map from function indices to a map from variables indices to names
    /// (mutable).
    pub fn local_names_mut(&mut self) -> &mut IndexMap<NameMap> {
        &mut self.local_names
    }
}

impl Serialize for LocalNameSection {
    type Error = Error;

    fn serialize<W: Write>(self, wtr: &mut W) -> Result<(), Error> {
        self.local_names.serialize(wtr)
    }
}

impl Deserialize for LocalNameSection {
    type Error = Error;

    fn deserialize<R: Read>(rdr: &mut R) -> Result<LocalNameSection, Error> {
        let local_names = IndexMap::deserialize(rdr)?;
        Ok(LocalNameSection { local_names })
    }
}

/// A map from indices to names.
pub type NameMap = IndexMap<String>;

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;

    // A helper funtion for the tests. Serialize a section, deserialize it,
    // and make sure it matches the original.
    fn serialize_and_deserialize(original: NameSection) {
        let mut buffer = vec![];
        original.clone().serialize(&mut buffer).expect("serialize error");
        let mut input = Cursor::new(buffer);
        let deserialized = NameSection::deserialize(&mut input)
            .expect("deserialize error");
        assert_eq!(deserialized, original);
    }

    #[test]
    fn serialize_and_deserialize_module_name() {
        let sect = ModuleNameSection::new("my_mod");
        serialize_and_deserialize(NameSection::Module(sect));
    }

    #[test]
    fn serialize_and_deserialize_function_names() {
        let mut sect = FunctionNameSection::default();
        sect.names_mut().insert(0, "hello_world".to_string());
        serialize_and_deserialize(NameSection::Function(sect));
    }

    #[test]
    fn serialize_and_deserialize_local_names() {
        let mut sect = LocalNameSection::default();
        let mut locals = NameMap::default();
        locals.insert(0, "msg".to_string());
        sect.local_names_mut().insert(0, locals);
        serialize_and_deserialize(NameSection::Local(sect));
    }

    #[test]
    fn serialize_and_deserialize_unparsed() {
        let sect = NameSection::Unparsed {
            // A made-up name section type which is unlikely to be allocated
            // soon.
            name_type: 120,
            name_payload: vec![0u8, 1, 2],
        };
        serialize_and_deserialize(sect);
    }
}
