//! Routines for parsing and reading structured data from resource forks
//!
//! ### Reference:
//!
//! [Inside Macintosh: More Macintosh Toolbox](https://archive.org/details/inside-macintosh-1992-1994/1993-more_macintosh_toolbox/)
//! Resource File Format 1-121 (pp. 151)

// Re: compressed resources: <http://preserve.mactech.com/articles/mactech/Vol.09/09.01/ResCompression/index.html>

#[cfg(feature = "no_std")]
use heapless::String;

use crate::binary::read::{
    CheckIndex, ReadArray, ReadBinary, ReadBinaryDep, ReadCtxt, ReadFrom, ReadScope,
};
use crate::binary::{I16Be, NumFrom, U16Be, U24Be, U32Be, U8};
use crate::error::ParseError;
use crate::macroman::FromMacRoman;
use crate::FourCC;

/// A parsed resource fork.
pub struct ResourceFork<'a> {
    rsrc_data: &'a [u8],
    map: ResourceMap<'a>,
}

struct ResourceMap<'a> {
    attributes: u16,
    type_list: TypeList<'a>,
    name_list_scope: ReadScope<'a>,
}

struct TypeList<'a> {
    scope: ReadScope<'a>,
    list: ReadArray<'a, TypeListItem>,
}

#[derive(Copy, Clone)]
pub struct TypeListItem {
    /// Resource type
    rsrc_type: FourCC,
    /// Number of resources of this type
    num_rsrc: u16,
    /// Offset from the beginning of the resource type list to reference list for this type
    reference_list_offset: u16,
}

struct ReferenceList<'a> {
    list: ReadArray<'a, ReferenceListItem>,
}

struct ReferenceListItem {
    id: i16,
    /// Offset from beginning of resource name list to resource name
    name_offset: Option<u16>,
    attributes: u8,
    /// Offset from beginning of resource data to data for this resource
    data_offset: u32, // actually only 3 bytes
}

/// An individual resource from a resource fork.
pub struct Resource<'a> {
    id: i16,
    name: Option<&'a [u8]>,
    attributes: u8,
    data: &'a [u8],
}

/// An iterator over the resource types in a resource fork.
///
/// Typically created with [ResourceFork::resource_types].
pub struct ResourceTypes<'a, 'rsrc> {
    fork: &'a ResourceFork<'rsrc>,
    type_index: u16,
}

/// An iterator over the resources of a given type.
///
/// Typically created with [ResourceFork::resources].
pub struct Resources<'a, 'rsrc> {
    fork: &'a ResourceFork<'rsrc>,
    item: TypeListItem,
    rsrc_index: u16,
}

impl<'a> ResourceFork<'a> {
    // FIXME: Make this a ReadBinary impl
    pub fn new(data: &[u8]) -> Result<ResourceFork<'_>, ParseError> {
        let scope = ReadScope::new(data);
        let mut ctxt = scope.ctxt();
        let data_offset = ctxt.read_u32be()?;
        let map_offset = ctxt.read_u32be()?;
        let data_len = ctxt.read_u32be()?;
        let map_len = ctxt.read_u32be()?;

        let rsrc_data =
            scope.offset_length(usize::num_from(data_offset), usize::num_from(data_len))?;
        let map_data =
            scope.offset_length(usize::num_from(map_offset), usize::num_from(map_len))?;
        let rsrc_map = map_data.read::<ResourceMap>()?;

        Ok(ResourceFork {
            rsrc_data: rsrc_data.data(),
            map: rsrc_map,
        })
    }

    pub fn resource_types(&self) -> ResourceTypes<'_, 'a> {
        ResourceTypes {
            fork: self,
            type_index: 0,
        }
    }

    pub fn resources<'b>(&'b self, item: TypeListItem) -> Resources<'_, 'a> {
        Resources {
            fork: self,
            item,
            rsrc_index: 0,
        }
    }
}

impl ResourceFork<'_> {
    pub fn get_resource(&self, rsrc_type: FourCC, rsrc_id: i16) -> Option<Resource<'_>> {
        let reference_list = self.map.type_list.find(rsrc_type)?;
        let item = reference_list.find(rsrc_id)?;
        self.read_resource(&item)
    }

    fn read_resource(&self, item: &ReferenceListItem) -> Option<Resource<'_>> {
        let data = self.read_resource_data(item.data_offset)?;
        let name = item.name_offset.and_then(|offset| self.read_name(offset));

        Some(Resource {
            id: item.id,
            name,
            attributes: item.attributes,
            data,
        })
    }

    fn read_resource_data(&self, offset: u32) -> Option<&[u8]> {
        let mut ctxt = ReadScope::new(self.rsrc_data)
            .offset(usize::num_from(offset))
            .ctxt();
        let len = ctxt.read_u32be().ok()?;
        ctxt.read_slice(usize::num_from(len)).ok() // FIXME: ok
    }

    fn read_name(&self, offset: u16) -> Option<&[u8]> {
        let mut ctxt = self.map.name_list_scope.offset(usize::from(offset)).ctxt();
        let len = ctxt.read_u8().ok()?;
        ctxt.read_slice(usize::from(len)).ok() // FIXME: ok
    }
}

impl ReadBinary for ResourceMap<'_> {
    type HostType<'a> = ResourceMap<'a>;

    fn read<'a>(ctxt: &mut ReadCtxt<'a>) -> Result<Self::HostType<'a>, ParseError> {
        // Skip the first 22 bytes these are all set to 0 and are used by the Resource
        // Manager for storing data at runtime.
        let scope = ctxt.scope();
        let _ = ctxt.read_slice(16 + 4 + 2)?;
        let attributes = ctxt.read_u16be()?;
        let rsrc_type_list_offset = ctxt.read_u16be()?;
        let rsrc_name_list_offset = ctxt.read_u16be()?;

        let type_list = scope
            .offset(usize::from(rsrc_type_list_offset))
            .read::<TypeList<'_>>()?;
        let name_list_scope = scope.offset(usize::from(rsrc_name_list_offset));

        Ok(ResourceMap {
            attributes,
            type_list,
            name_list_scope,
        })
    }
}

impl ReadBinary for TypeList<'_> {
    type HostType<'a> = TypeList<'a>;

    fn read<'a>(ctxt: &mut ReadCtxt<'a>) -> Result<Self::HostType<'a>, ParseError> {
        let scope = ctxt.scope();
        // Value is stored minus 1, so add 1 to it after reading
        let num_types = ctxt
            .read_u16be()?
            .checked_add(1)
            .ok_or(ParseError::Overflow)?;
        let list = ctxt.read_array::<TypeListItem>(usize::from(num_types))?;

        Ok(TypeList { scope, list })
    }
}

impl TypeList<'_> {
    fn find(&self, rsrc_type: FourCC) -> Option<ReferenceList<'_>> {
        let item = self.list.iter().find(|item| item.rsrc_type == rsrc_type)?;
        item.reference_list(self.scope)
    }
}

impl ReadFrom for TypeListItem {
    type ReadType = (FourCC, U16Be, U16Be);

    fn from((rsrc_type, num_rsrc, reference_list_offset): (FourCC, u16, u16)) -> Self {
        TypeListItem {
            rsrc_type,
            // Value is stored minus 1
            num_rsrc: num_rsrc.wrapping_add(1),
            reference_list_offset,
        }
    }
}

impl TypeListItem {
    pub fn resource_type(&self) -> FourCC {
        self.rsrc_type
    }

    fn reference_list<'a>(&self, scope: ReadScope<'a>) -> Option<ReferenceList<'a>> {
        scope
            .offset(usize::from(self.reference_list_offset))
            .read_dep::<ReferenceList<'_>>(self.num_rsrc)
            .ok() // FIXME: ok?
    }
}

impl ReadBinaryDep for ReferenceList<'_> {
    type Args<'a> = u16;
    type HostType<'a> = ReferenceList<'a>;

    fn read_dep<'a>(
        ctxt: &mut ReadCtxt<'a>,
        num_rsrc: u16,
    ) -> Result<Self::HostType<'a>, ParseError> {
        let list = ctxt.read_array::<ReferenceListItem>(usize::from(num_rsrc))?;
        Ok(ReferenceList { list })
    }
}

impl ReferenceList<'_> {
    fn find(&self, id: i16) -> Option<ReferenceListItem> {
        self.list.iter().find(|item| item.id == id)
    }
}

impl ReadFrom for ReferenceListItem {
    type ReadType = ((I16Be, I16Be, U8), U24Be, U32Be);

    fn from(
        ((id, name_offset, attributes), data_offset, _reserved): ((i16, i16, u8), u32, u32),
    ) -> Self {
        ReferenceListItem {
            id,
            name_offset: (name_offset >= 0).then_some(name_offset as u16),
            attributes,
            data_offset,
        }
    }
}

impl Resource<'_> {
    pub fn id(&self) -> i16 {
        self.id
    }

    #[cfg(not(feature = "no_std"))]
    pub fn name(&self) -> Option<String> {
        self.name.map(|name| String::from_macroman(name))
    }

    /// The name associated with this resource, if present.
    ///
    /// The raw name can't be longer than 255 bytes as the length is specified with a byte. However,
    /// this method converts the raw bytes from MacRoman into UTF-8 string and many non-ASCII
    /// MacRoman bytes encode to more than one byte in UTF-8. This method will return `None` if
    /// the `N` parameter is too small to hold the UTF-8 string.
    #[cfg(feature = "no_std")]
    pub fn name<const N: usize>(&self) -> Option<String<N>> {
        self.name.and_then(String::try_from_macroman)
    }

    /// The raw bytes of the resource name.
    pub fn name_bytes(&self) -> Option<&[u8]> {
        self.name
    }

    pub fn data(&self) -> &[u8] {
        self.data
    }
}

impl<'a, 'rsrc> Iterator for ResourceTypes<'a, 'rsrc> {
    type Item = TypeListItem;

    fn next(&mut self) -> Option<Self::Item> {
        // Get the current type list
        let list = &self.fork.map.type_list.list;
        let type_list_item = list
            .check_index(usize::from(self.type_index))
            .ok()
            .map(|()| list.get_item(usize::from(self.type_index)))?;

        self.type_index += 1;
        Some(type_list_item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let num_remaining = self.fork.map.type_list.list.len() - usize::from(self.type_index);
        (num_remaining, Some(num_remaining))
    }
}

impl<'rsrc, 'a: 'rsrc> Iterator for Resources<'a, 'rsrc> {
    type Item = Resource<'rsrc>;

    fn next(&mut self) -> Option<Self::Item> {
        let reference_list = self.reference_list()?;
        let reference_list_item = reference_list
            .list
            .check_index(usize::from(self.rsrc_index))
            .ok()
            .map(|()| reference_list.list.get_item(usize::from(self.rsrc_index)))?;
        let resource = self.fork.read_resource(&reference_list_item)?;

        self.rsrc_index += 1;
        Some(resource)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.reference_list()
            .map(|reference_list| {
                let num_remaining = reference_list.list.len() - usize::from(self.rsrc_index);
                (num_remaining, Some(num_remaining))
            })
            .unwrap_or((0, None))
    }
}

impl Resources<'_, '_> {
    fn reference_list(&self) -> Option<ReferenceList<'_>> {
        self.item.reference_list(self.fork.map.type_list.scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::read_fixture;

    #[test]
    fn test_macbinary_3() {
        let data = read_fixture("tests/Text File.bin");
        let file = crate::parse(&data).unwrap();
        let rsrc = ResourceFork::new(file.resource_fork_raw()).unwrap();
        let bbst = rsrc
            .get_resource(FourCC(u32::from_be_bytes(*b"BBST")), 128)
            .unwrap();
        assert_eq!(bbst.data().len(), 1048);

        let mpsr = rsrc
            .get_resource(FourCC(u32::from_be_bytes(*b"MPSR")), 1005)
            .unwrap();
        assert_eq!(
            mpsr.data(),
            &[
                0x00, 0x09, 0x4D, 0x6F, 0x6E, 0x61, 0x63, 0x6F, 0x00, 0xE0, 0x00, 0x00, 0x00, 0x00,
                0x07, 0x10, 0xA6, 0xF0, 0x00, 0x07, 0x07, 0x10, 0xC0, 0xA8, 0x06, 0xFA, 0x94, 0x40,
                0x07, 0x10, 0xA7, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x04, 0x00, 0x2C, 0x00, 0x36,
                0x02, 0xF7, 0x02, 0xB6, 0x00, 0x2C, 0x00, 0x36, 0x02, 0xF7, 0x02, 0xB6, 0xE0, 0x40,
                0xD4, 0xE8, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00,
                0x01, 0x00
            ]
        );
    }

    #[test]
    fn test_iter_types() {
        let data = read_fixture("tests/Text File.bin");
        let file = crate::parse(&data).unwrap();
        let rsrc = ResourceFork::new(file.resource_fork_raw()).unwrap();
        let types: Vec<_> = rsrc
            .resource_types()
            .map(|item| item.resource_type().to_string())
            .collect();
        assert_eq!(types, vec![String::from("MPSR"), String::from("BBST")]);
    }

    #[test]
    fn test_iter_resources() {
        let data = read_fixture("tests/Text File.bin");
        let file = crate::parse(&data).unwrap();
        let rsrc = file.resource_fork().unwrap();
        let mut resources = Vec::new();
        for item in rsrc.resource_types() {
            resources.extend(rsrc.resources(item).map(|resource| {
                (
                    item.rsrc_type.to_string(),
                    resource.id,
                    resource.name(),
                    resource.data().len(),
                )
            }))
        }
        assert_eq!(
            resources,
            vec![
                (String::from("MPSR"), 1005, None, 72),
                (String::from("BBST"), 128, None, 1048),
            ]
        );
    }
}
