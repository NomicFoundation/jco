use std::collections::HashMap;

use serde::Deserialize;
use wit_parser::{Case, Function, Handle, Resolve, Type, TypeDef, TypeDefKind, TypeId, TypeOwner};

pub trait ConfigurableElement {
    fn configuration_name(&self, resolve: &Resolve) -> String;
}

impl ConfigurableElement for &TypeId {
    fn configuration_name(&self, resolve: &Resolve) -> String {
        let type_ = &resolve.types[**self];
        let mut path = vec![];
        match type_.owner {
            wit_parser::TypeOwner::World(world_id) => {
                path.push(resolve.worlds[world_id].name.clone())
            }
            wit_parser::TypeOwner::Interface(interface_id) => {
                let interface = &resolve.interfaces[interface_id];
                if let Some(package_id) = interface.package {
                    let package = &resolve.packages[package_id];
                    path.push(package.name.namespace.clone());
                    path.push(package.name.name.clone());
                }
                if let Some(name) = interface.name.as_ref() {
                    path.push(name.clone());
                }
            }
            wit_parser::TypeOwner::None => {}
        }
        if let Some(name) = type_.name.as_ref() {
            path.push(name.clone());
        }
        path.join(":")
    }
}

impl ConfigurableElement for &Function {
    fn configuration_name(&self, resolve: &Resolve) -> String {
        match self.kind {
            wit_parser::FunctionKind::Freestanding => {
                // TODO: need the world for this case
                self.item_name().to_string()
            }
            wit_parser::FunctionKind::Method(type_id)
            | wit_parser::FunctionKind::Static(type_id)
            | wit_parser::FunctionKind::Constructor(type_id) => {
                let path = (&type_id).configuration_name(resolve);
                let name = self.item_name();
                format!("{path}.{name}()")
            }
        }
    }
}

#[derive(Default, Deserialize, Clone, Debug)]
pub struct Configuration {
    mappings: HashMap<String, ElementConfig>,
}

impl Configuration {
    pub(crate) fn get<E: ConfigurableElement>(
        &self,
        resolve: &Resolve,
        element: E,
    ) -> &ElementConfig {
        let path = element.configuration_name(resolve);
        self.mappings.get(&path).unwrap_or(&ElementConfig::None)
    }

    pub(crate) fn get_member<E: ConfigurableElement>(
        &self,
        resolve: &Resolve,
        element: E,
        name: &String,
    ) -> &ElementConfig {
        let path = element.configuration_name(resolve);
        self.mappings
            .get(&format!("{path}.{name}"))
            .unwrap_or(&ElementConfig::None)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub enum ElementConfig {
    None,
    Record {
        #[serde(default)]
        as_class: bool,
    },
    Resource {
        #[serde(default)]
        as_iterator: bool,
        #[serde(default)]
        use_guest_class: bool,
    },
    Enum {
        #[serde(default)]
        as_typescript_enum: bool,
    },
    Variant {
        #[serde(default)]
        as_direct_union_of_resource_classes: bool,
    },
    ListOfTuple {
        #[serde(default)]
        as_dictionary: bool,
    },
}

impl ElementConfig {
    pub fn enum_as_typescript_enum(&self) -> bool {
        match self {
            ElementConfig::Enum { as_typescript_enum } => *as_typescript_enum,
            _ => false,
        }
    }

    pub fn record_as_class(&self) -> bool {
        match self {
            ElementConfig::Record { as_class } => *as_class,
            _ => false,
        }
    }

    pub fn resource_as_iterator(&self) -> bool {
        match self {
            ElementConfig::Resource { as_iterator, .. } => *as_iterator,
            _ => false,
        }
    }

    pub fn resource_use_guest_class(&self) -> bool {
        match self {
            ElementConfig::Resource {
                use_guest_class, ..
            } => *use_guest_class,
            _ => false,
        }
    }

    pub fn variant_as_direct_union_of_resource_classes(&self) -> bool {
        match self {
            ElementConfig::Variant {
                as_direct_union_of_resource_classes: as_union_of_classes,
            } => *as_union_of_classes,
            _ => false,
        }
    }

    pub fn list_of_tuple_as_dictionary(&self) -> bool {
        match self {
            ElementConfig::ListOfTuple { as_dictionary } => *as_dictionary,
            _ => false,
        }
    }
}

trait PrivateTypeExtensions {
    fn type_id(&self) -> Option<TypeId>;
    fn type_def<'a>(&self, resolve: &'a Resolve) -> Option<&'a TypeDef>;
    fn list_element_type(&self, resolve: &Resolve) -> Option<Type>;
    fn option_payload_type(&self, resolve: &Resolve) -> Option<Type>;
    fn tuple_types(&self, resolve: &Resolve) -> Option<Vec<Type>>;
    fn variant_cases(&self, resolve: &Resolve) -> Option<Vec<Case>>;
    // fn owning_interface<'a>(&self, resolve: &'a Resolve) -> Option<&'a Interface>;
    fn methods_of_resource<'a>(&self, resolve: &'a Resolve) -> Option<Vec<&'a Function>>;
}

impl PrivateTypeExtensions for Type {
    fn type_id<'a>(&'a self) -> Option<TypeId> {
        if let Type::Id(type_id) = self {
            Some(*type_id)
        } else {
            None
        }
    }

    fn type_def<'a>(&self, resolve: &'a Resolve) -> Option<&'a TypeDef> {
        self.type_id().map(|type_id| &resolve.types[type_id])
    }

    fn list_element_type(&self, resolve: &Resolve) -> Option<Type> {
        self.type_def(resolve).and_then(|type_def| {
            if let TypeDefKind::List(ty) = &type_def.kind {
                Some(*ty)
            } else {
                None
            }
        })
    }

    fn option_payload_type(&self, resolve: &Resolve) -> Option<Type> {
        self.type_def(resolve).and_then(|type_def| {
            if let TypeDefKind::Option(ty) = &type_def.kind {
                Some(*ty)
            } else {
                None
            }
        })
    }

    fn tuple_types(&self, resolve: &Resolve) -> Option<Vec<Type>> {
        self.type_def(resolve).and_then(|type_def| {
            if let TypeDefKind::Tuple(tuple) = &type_def.kind {
                Some(tuple.types.clone())
            } else {
                None
            }
        })
    }

    fn variant_cases(&self, resolve: &Resolve) -> Option<Vec<Case>> {
        self.type_def(resolve).and_then(|type_def| {
            if let TypeDefKind::Variant(variant) = &type_def.kind {
                Some(variant.cases.clone())
            } else {
                None
            }
        })
    }

    // fn owning_interface<'a>(&self, resolve: &'a Resolve) -> Option<&'a Interface> {
    //     self.type_def(resolve).and_then(|type_def| {
    //         if let TypeOwner::Interface(interface_id) = type_def.owner {
    //             Some(&resolve.interfaces[interface_id])
    //         } else {
    //             None
    //         }
    //     })
    // }

    fn methods_of_resource<'a>(&self, resolve: &'a Resolve) -> Option<Vec<&'a Function>> {
        self.type_id().and_then(|type_id| {
            let type_def = &resolve.types[type_id];
            if let TypeOwner::Interface(interface_id) = type_def.owner {
                let interface = &resolve.interfaces[interface_id];
                Some(
                    interface
                        .functions
                        .iter()
                        .flat_map(|(_, function)| {
                            if wit_parser::FunctionKind::Method(type_id) == function.kind {
                                Some(function)
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
            } else {
                None
            }
        })
    }
}

pub trait TypeExtensions {
    fn value_type_of_list_of_tuple_interpretable_as_dictionary(
        &self,
        resolve: &Resolve,
    ) -> Option<Type>;

    fn payload_type_of_option_result_of_next_method_of_resource(
        &self,
        resolve: &Resolve,
    ) -> Option<Type>;

    fn variant_case_type_defs_where_they_are_all_handles<'a>(
        &self,
        resolve: &'a Resolve,
    ) -> Option<Vec<&'a TypeDef>>;
}

impl TypeExtensions for Type {
    fn value_type_of_list_of_tuple_interpretable_as_dictionary(
        &self,
        resolve: &Resolve,
    ) -> Option<Type> {
        self.list_element_type(resolve)
            .and_then(|ty| ty.tuple_types(resolve))
            .and_then(|tuple_types| {
                if tuple_types.len() == 2 && tuple_types[0] == Type::String {
                    return Some(tuple_types[1]);
                } else {
                    None
                }
            })
    }

    fn payload_type_of_option_result_of_next_method_of_resource(
        &self,
        resolve: &Resolve,
    ) -> Option<Type> {
        self.methods_of_resource(resolve).and_then(|methods| {
            methods
                .into_iter()
                .find(|method| method.item_name() == "next")
                .and_then(|method| {
                    if method.results.len() == 1 {
                        method
                            .results
                            .iter_types()
                            .next()
                            .and_then(|ty| ty.option_payload_type(resolve))
                    } else {
                        None
                    }
                })
        })
    }

    fn variant_case_type_defs_where_they_are_all_handles<'a>(
        &self,
        resolve: &'a Resolve,
    ) -> Option<Vec<&'a TypeDef>> {
        self.variant_cases(resolve).map(|cases| {
            cases
                .iter()
                .filter_map(|case| {
                    case.ty.and_then(|ty| {
                        ty.type_def(resolve).and_then(|type_def| {
                            if let TypeDefKind::Handle(Handle::Own(type_id)) = type_def.kind {
                                Type::Id(type_id).type_def(resolve)
                            } else {
                                None
                            }
                        })
                    })
                })
                .collect()
        })
    }
}
