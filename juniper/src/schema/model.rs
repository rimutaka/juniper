use std::{borrow::Cow, fmt};

use fnv::FnvHashMap;
#[cfg(feature = "graphql-parser")]
use graphql_parser::schema::Document;

use crate::{
    ast::Type,
    executor::{Context, Registry},
    schema::meta::{Argument, InterfaceMeta, MetaType, ObjectMeta, PlaceholderMeta, UnionMeta},
    types::{base::GraphQLType, name::Name},
    value::{DefaultScalarValue, ScalarValue},
    GraphQLEnum,
};

#[cfg(feature = "graphql-parser")]
use crate::schema::translate::{graphql_parser::GraphQLParserTranslator, SchemaTranslator};

/// Root query node of a schema
///
/// This brings the mutation, subscription and query types together,
/// and provides the predefined metadata fields.
#[derive(Debug)]
pub struct RootNode<
    'a,
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S>,
    SubscriptionT: GraphQLType<S>,
    S = DefaultScalarValue,
> where
    S: ScalarValue,
{
    #[doc(hidden)]
    pub query_type: QueryT,
    #[doc(hidden)]
    pub query_info: QueryT::TypeInfo,
    #[doc(hidden)]
    pub mutation_type: MutationT,
    #[doc(hidden)]
    pub mutation_info: MutationT::TypeInfo,
    #[doc(hidden)]
    pub subscription_type: SubscriptionT,
    #[doc(hidden)]
    pub subscription_info: SubscriptionT::TypeInfo,
    #[doc(hidden)]
    pub schema: SchemaType<'a, S>,
}

/// Metadata for a schema
#[derive(Debug)]
pub struct SchemaType<'a, S> {
    pub(crate) description: Option<Cow<'a, str>>,
    pub(crate) types: FnvHashMap<Name, MetaType<'a, S>>,
    pub(crate) query_type_name: String,
    pub(crate) mutation_type_name: Option<String>,
    pub(crate) subscription_type_name: Option<String>,
    directives: FnvHashMap<String, DirectiveType<'a, S>>,
}

impl<'a, S> Context for SchemaType<'a, S> {}

#[derive(Clone)]
pub enum TypeType<'a, S: 'a> {
    Concrete(&'a MetaType<'a, S>),
    NonNull(Box<TypeType<'a, S>>),
    List(Box<TypeType<'a, S>>, Option<usize>),
}

#[derive(Debug)]
pub struct DirectiveType<'a, S> {
    pub name: String,
    pub description: Option<String>,
    pub locations: Vec<DirectiveLocation>,
    pub arguments: Vec<Argument<'a, S>>,
    pub is_repeatable: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, GraphQLEnum)]
#[graphql(name = "__DirectiveLocation", internal)]
pub enum DirectiveLocation {
    Query,
    Mutation,
    Subscription,
    Field,
    Scalar,
    #[graphql(name = "FRAGMENT_DEFINITION")]
    FragmentDefinition,
    #[graphql(name = "FIELD_DEFINITION")]
    FieldDefinition,
    #[graphql(name = "VARIABLE_DEFINITION")]
    VariableDefinition,
    #[graphql(name = "FRAGMENT_SPREAD")]
    FragmentSpread,
    #[graphql(name = "INLINE_FRAGMENT")]
    InlineFragment,
    #[graphql(name = "ENUM_VALUE")]
    EnumValue,
}

impl<'a, QueryT, MutationT, SubscriptionT>
    RootNode<'a, QueryT, MutationT, SubscriptionT, DefaultScalarValue>
where
    QueryT: GraphQLType<DefaultScalarValue, TypeInfo = ()>,
    MutationT: GraphQLType<DefaultScalarValue, TypeInfo = ()>,
    SubscriptionT: GraphQLType<DefaultScalarValue, TypeInfo = ()>,
{
    /// Constructs a new [`RootNode`] from `query`, `mutation` and `subscription` nodes,
    /// parametrizing it with a [`DefaultScalarValue`].
    pub fn new(query: QueryT, mutation: MutationT, subscription: SubscriptionT) -> Self {
        Self::new_with_info(query, mutation, subscription, (), (), ())
    }
}

impl<'a, QueryT, MutationT, SubscriptionT, S> RootNode<'a, QueryT, MutationT, SubscriptionT, S>
where
    S: ScalarValue + 'a,
    QueryT: GraphQLType<S, TypeInfo = ()>,
    MutationT: GraphQLType<S, TypeInfo = ()>,
    SubscriptionT: GraphQLType<S, TypeInfo = ()>,
{
    /// Constructs a new [`RootNode`] from `query`, `mutation` and `subscription` nodes,
    /// parametrizing it with the provided [`ScalarValue`].
    pub fn new_with_scalar_value(
        query: QueryT,
        mutation: MutationT,
        subscription: SubscriptionT,
    ) -> Self {
        RootNode::new_with_info(query, mutation, subscription, (), (), ())
    }
}

impl<'a, S, QueryT, MutationT, SubscriptionT> RootNode<'a, QueryT, MutationT, SubscriptionT, S>
where
    QueryT: GraphQLType<S>,
    MutationT: GraphQLType<S>,
    SubscriptionT: GraphQLType<S>,
    S: ScalarValue + 'a,
{
    /// Construct a new root node from query and mutation nodes,
    /// while also providing type info objects for the query and
    /// mutation types.
    pub fn new_with_info(
        query_obj: QueryT,
        mutation_obj: MutationT,
        subscription_obj: SubscriptionT,
        query_info: QueryT::TypeInfo,
        mutation_info: MutationT::TypeInfo,
        subscription_info: SubscriptionT::TypeInfo,
    ) -> Self {
        RootNode {
            query_type: query_obj,
            mutation_type: mutation_obj,
            subscription_type: subscription_obj,
            schema: SchemaType::new::<QueryT, MutationT, SubscriptionT>(
                &query_info,
                &mutation_info,
                &subscription_info,
            ),
            query_info,
            mutation_info,
            subscription_info,
        }
    }

    #[cfg(feature = "schema-language")]
    /// The schema definition as a `String` in the
    /// [GraphQL Schema Language](https://graphql.org/learn/schema/#type-language)
    /// format.
    pub fn as_schema_language(&self) -> String {
        self.as_parser_document().to_string()
    }

    #[cfg(feature = "graphql-parser")]
    /// The schema definition as a [`graphql_parser`](https://crates.io/crates/graphql-parser)
    /// [`Document`](https://docs.rs/graphql-parser/latest/graphql_parser/schema/struct.Document.html).
    pub fn as_parser_document(&'a self) -> Document<'a, &'a str> {
        GraphQLParserTranslator::translate_schema(&self.schema)
    }
}

impl<'a, S> SchemaType<'a, S> {
    /// Create a new schema.
    pub fn new<QueryT, MutationT, SubscriptionT>(
        query_info: &QueryT::TypeInfo,
        mutation_info: &MutationT::TypeInfo,
        subscription_info: &SubscriptionT::TypeInfo,
    ) -> Self
    where
        S: ScalarValue + 'a,
        QueryT: GraphQLType<S>,
        MutationT: GraphQLType<S>,
        SubscriptionT: GraphQLType<S>,
    {
        let mut directives = FnvHashMap::default();
        let mut registry = Registry::new(FnvHashMap::default());

        let query_type_name = registry
            .get_type::<QueryT>(query_info)
            .innermost_name()
            .to_owned();
        let mutation_type_name = registry
            .get_type::<MutationT>(mutation_info)
            .innermost_name()
            .to_owned();
        let subscription_type_name = registry
            .get_type::<SubscriptionT>(subscription_info)
            .innermost_name()
            .to_owned();

        registry.get_type::<SchemaType<S>>(&());

        directives.insert("skip".into(), DirectiveType::new_skip(&mut registry));
        directives.insert("include".into(), DirectiveType::new_include(&mut registry));
        directives.insert(
            "deprecated".into(),
            DirectiveType::new_deprecated(&mut registry),
        );
        directives.insert(
            "specifiedBy".into(),
            DirectiveType::new_specified_by(&mut registry),
        );

        let mut meta_fields = vec![
            registry.field::<SchemaType<S>>("__schema", &()),
            registry
                .field::<TypeType<S>>("__type", &())
                .argument(registry.arg::<String>("name", &())),
        ];

        if let Some(root_type) = registry.types.get_mut(&query_type_name) {
            if let MetaType::Object(ObjectMeta { ref mut fields, .. }) = *root_type {
                fields.append(&mut meta_fields);
            } else {
                panic!("Root type is not an object");
            }
        } else {
            panic!("Root type not found");
        }

        for meta_type in registry.types.values() {
            if let MetaType::Placeholder(PlaceholderMeta { ref of_type }) = *meta_type {
                panic!("Type {of_type:?} is still a placeholder type");
            }
        }
        SchemaType {
            description: None,
            types: registry.types,
            query_type_name,
            mutation_type_name: if &mutation_type_name != "_EmptyMutation" {
                Some(mutation_type_name)
            } else {
                None
            },
            subscription_type_name: if &subscription_type_name != "_EmptySubscription" {
                Some(subscription_type_name)
            } else {
                None
            },
            directives,
        }
    }

    /// Add a description.
    pub fn set_description(&mut self, description: impl Into<Cow<'a, str>>) {
        self.description = Some(description.into());
    }

    /// Add a directive like `skip` or `include`.
    pub fn add_directive(&mut self, directive: DirectiveType<'a, S>) {
        self.directives.insert(directive.name.clone(), directive);
    }

    /// Get a type by name.
    pub fn type_by_name(&self, name: &str) -> Option<TypeType<S>> {
        self.types.get(name).map(|t| TypeType::Concrete(t))
    }

    /// Get a concrete type by name.
    pub fn concrete_type_by_name(&self, name: &str) -> Option<&MetaType<S>> {
        self.types.get(name)
    }

    pub(crate) fn lookup_type(&self, tpe: &Type) -> Option<&MetaType<S>> {
        match *tpe {
            Type::NonNullNamed(ref name) | Type::Named(ref name) => {
                self.concrete_type_by_name(name)
            }
            Type::List(ref inner, _) | Type::NonNullList(ref inner, _) => self.lookup_type(inner),
        }
    }

    /// Get the query type from the schema.
    pub fn query_type(&self) -> TypeType<S> {
        TypeType::Concrete(
            self.types
                .get(&self.query_type_name)
                .expect("Query type does not exist in schema"),
        )
    }

    /// Get the concrete query type from the schema.
    pub fn concrete_query_type(&self) -> &MetaType<S> {
        self.types
            .get(&self.query_type_name)
            .expect("Query type does not exist in schema")
    }

    /// Get the mutation type from the schema.
    pub fn mutation_type(&self) -> Option<TypeType<S>> {
        self.mutation_type_name.as_ref().map(|name| {
            self.type_by_name(name)
                .expect("Mutation type does not exist in schema")
        })
    }

    /// Get the concrete mutation type from the schema.
    pub fn concrete_mutation_type(&self) -> Option<&MetaType<S>> {
        self.mutation_type_name.as_ref().map(|name| {
            self.concrete_type_by_name(name)
                .expect("Mutation type does not exist in schema")
        })
    }

    /// Get the subscription type.
    pub fn subscription_type(&self) -> Option<TypeType<S>> {
        self.subscription_type_name.as_ref().map(|name| {
            self.type_by_name(name)
                .expect("Subscription type does not exist in schema")
        })
    }

    /// Get the concrete subscription type.
    pub fn concrete_subscription_type(&self) -> Option<&MetaType<S>> {
        self.subscription_type_name.as_ref().map(|name| {
            self.concrete_type_by_name(name)
                .expect("Subscription type does not exist in schema")
        })
    }

    /// Get a list of types.
    pub fn type_list(&self) -> Vec<TypeType<S>> {
        self.types.values().map(|t| TypeType::Concrete(t)).collect()
    }

    /// Get a list of concrete types.
    pub fn concrete_type_list(&self) -> Vec<&MetaType<S>> {
        self.types.values().collect()
    }

    /// Make a type.
    pub fn make_type(&self, t: &Type) -> TypeType<S> {
        match *t {
            Type::NonNullNamed(ref n) => TypeType::NonNull(Box::new(
                self.type_by_name(n).expect("Type not found in schema"),
            )),
            Type::NonNullList(ref inner, expected_size) => TypeType::NonNull(Box::new(
                TypeType::List(Box::new(self.make_type(inner)), expected_size),
            )),
            Type::Named(ref n) => self.type_by_name(n).expect("Type not found in schema"),
            Type::List(ref inner, expected_size) => {
                TypeType::List(Box::new(self.make_type(inner)), expected_size)
            }
        }
    }

    /// Get a list of directives.
    pub fn directive_list(&self) -> Vec<&DirectiveType<S>> {
        self.directives.values().collect()
    }

    /// Get directive by name.
    pub fn directive_by_name(&self, name: &str) -> Option<&DirectiveType<S>> {
        self.directives.get(name)
    }

    /// Determine if there is an overlap between types.
    pub fn type_overlap(&self, t1: &MetaType<S>, t2: &MetaType<S>) -> bool {
        if std::ptr::eq(t1, t2) {
            return true;
        }

        match (t1.is_abstract(), t2.is_abstract()) {
            (true, true) => self
                .possible_types(t1)
                .iter()
                .any(|t| self.is_possible_type(t2, t)),
            (true, false) => self.is_possible_type(t1, t2),
            (false, true) => self.is_possible_type(t2, t1),
            (false, false) => false,
        }
    }

    /// A list of possible typeees for a given type.
    pub fn possible_types(&self, t: &MetaType<S>) -> Vec<&MetaType<S>> {
        match *t {
            MetaType::Union(UnionMeta {
                ref of_type_names, ..
            }) => of_type_names
                .iter()
                .flat_map(|t| self.concrete_type_by_name(t))
                .collect(),
            MetaType::Interface(InterfaceMeta { ref name, .. }) => self
                .concrete_type_list()
                .into_iter()
                .filter(|t| match **t {
                    MetaType::Object(ObjectMeta {
                        ref interface_names,
                        ..
                    }) => interface_names.iter().any(|iname| iname == name),
                    _ => false,
                })
                .collect(),
            _ => panic!("Can't retrieve possible types from non-abstract meta type"),
        }
    }

    /// If the abstract type is possible.
    pub fn is_possible_type(
        &self,
        abstract_type: &MetaType<S>,
        possible_type: &MetaType<S>,
    ) -> bool {
        self.possible_types(abstract_type)
            .into_iter()
            .any(|t| (std::ptr::eq(t, possible_type)))
    }

    /// If the type is a subtype of another type.
    pub fn is_subtype<'b>(&self, sub_type: &Type<'b>, super_type: &Type<'b>) -> bool {
        use crate::ast::Type::*;

        if super_type == sub_type {
            return true;
        }

        match (super_type, sub_type) {
            (&NonNullNamed(ref super_name), &NonNullNamed(ref sub_name))
            | (&Named(ref super_name), &Named(ref sub_name))
            | (&Named(ref super_name), &NonNullNamed(ref sub_name)) => {
                self.is_named_subtype(sub_name, super_name)
            }
            (&NonNullList(ref super_inner, _), &NonNullList(ref sub_inner, _))
            | (&List(ref super_inner, _), &List(ref sub_inner, _))
            | (&List(ref super_inner, _), &NonNullList(ref sub_inner, _)) => {
                self.is_subtype(sub_inner, super_inner)
            }
            _ => false,
        }
    }

    /// If the type is a named subtype.
    pub fn is_named_subtype(&self, sub_type_name: &str, super_type_name: &str) -> bool {
        if sub_type_name == super_type_name {
            true
        } else if let (Some(sub_type), Some(super_type)) = (
            self.concrete_type_by_name(sub_type_name),
            self.concrete_type_by_name(super_type_name),
        ) {
            super_type.is_abstract() && self.is_possible_type(super_type, sub_type)
        } else {
            false
        }
    }
}

impl<'a, S> TypeType<'a, S> {
    #[inline]
    pub fn to_concrete(&self) -> Option<&'a MetaType<S>> {
        match *self {
            TypeType::Concrete(t) => Some(t),
            _ => None,
        }
    }

    #[inline]
    pub fn innermost_concrete(&self) -> &'a MetaType<S> {
        match *self {
            TypeType::Concrete(t) => t,
            TypeType::NonNull(ref n) | TypeType::List(ref n, _) => n.innermost_concrete(),
        }
    }

    #[inline]
    pub fn list_contents(&self) -> Option<&TypeType<'a, S>> {
        match *self {
            TypeType::List(ref n, _) => Some(n),
            TypeType::NonNull(ref n) => n.list_contents(),
            _ => None,
        }
    }

    #[inline]
    pub fn is_non_null(&self) -> bool {
        matches!(*self, TypeType::NonNull(_))
    }
}

impl<'a, S> DirectiveType<'a, S>
where
    S: ScalarValue + 'a,
{
    pub fn new(
        name: &str,
        locations: &[DirectiveLocation],
        arguments: &[Argument<'a, S>],
        is_repeatable: bool,
    ) -> Self {
        Self {
            name: name.into(),
            description: None,
            locations: locations.to_vec(),
            arguments: arguments.to_vec(),
            is_repeatable,
        }
    }

    fn new_skip(registry: &mut Registry<'a, S>) -> DirectiveType<'a, S>
    where
        S: ScalarValue,
    {
        Self::new(
            "skip",
            &[
                DirectiveLocation::Field,
                DirectiveLocation::FragmentSpread,
                DirectiveLocation::InlineFragment,
            ],
            &[registry.arg::<bool>("if", &())],
            false,
        )
    }

    fn new_include(registry: &mut Registry<'a, S>) -> DirectiveType<'a, S>
    where
        S: ScalarValue,
    {
        Self::new(
            "include",
            &[
                DirectiveLocation::Field,
                DirectiveLocation::FragmentSpread,
                DirectiveLocation::InlineFragment,
            ],
            &[registry.arg::<bool>("if", &())],
            false,
        )
    }

    fn new_deprecated(registry: &mut Registry<'a, S>) -> DirectiveType<'a, S>
    where
        S: ScalarValue,
    {
        Self::new(
            "deprecated",
            &[
                DirectiveLocation::FieldDefinition,
                DirectiveLocation::EnumValue,
            ],
            &[registry.arg::<String>("reason", &())],
            false,
        )
    }

    fn new_specified_by(registry: &mut Registry<'a, S>) -> DirectiveType<'a, S>
    where
        S: ScalarValue,
    {
        Self::new(
            "specifiedBy",
            &[DirectiveLocation::Scalar],
            &[registry.arg::<String>("url", &())],
            false,
        )
    }

    pub fn description(mut self, description: &str) -> DirectiveType<'a, S> {
        self.description = Some(description.into());
        self
    }
}

impl fmt::Display for DirectiveLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Self::Query => "query",
            Self::Mutation => "mutation",
            Self::Subscription => "subscription",
            Self::Field => "field",
            Self::FieldDefinition => "field definition",
            Self::FragmentDefinition => "fragment definition",
            Self::FragmentSpread => "fragment spread",
            Self::InlineFragment => "inline fragment",
            Self::VariableDefinition => "variable definition",
            Self::Scalar => "scalar",
            Self::EnumValue => "enum value",
        })
    }
}

impl<'a, S> fmt::Display for TypeType<'a, S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Concrete(t) => f.write_str(t.name().unwrap()),
            Self::List(i, _) => write!(f, "[{i}]"),
            Self::NonNull(i) => write!(f, "{i}!"),
        }
    }
}

#[cfg(test)]
mod test {

    #[cfg(feature = "graphql-parser")]
    mod graphql_parser_integration {
        use crate::{graphql_object, EmptyMutation, EmptySubscription, RootNode};

        #[test]
        fn graphql_parser_doc() {
            struct Query;
            #[graphql_object]
            impl Query {
                fn blah() -> bool {
                    true
                }
            }
            let schema = RootNode::new(
                Query,
                EmptyMutation::<()>::new(),
                EmptySubscription::<()>::new(),
            );
            let ast = graphql_parser::parse_schema::<&str>(
                r#"
                type Query {
                  blah: Boolean!
                }

                schema {
                  query: Query
                }
            "#,
            )
            .unwrap();
            assert_eq!(ast.to_string(), schema.as_parser_document().to_string());
        }
    }

    #[cfg(feature = "schema-language")]
    mod schema_language {
        use crate::{
            graphql_object, EmptyMutation, EmptySubscription, GraphQLEnum, GraphQLInputObject,
            GraphQLObject, GraphQLUnion, RootNode,
        };

        #[test]
        fn schema_language() {
            #[derive(GraphQLObject, Default)]
            struct Cake {
                fresh: bool,
            }
            #[derive(GraphQLObject, Default)]
            struct IceCream {
                cold: bool,
            }
            #[derive(GraphQLUnion)]
            enum GlutenFree {
                Cake(Cake),
                IceCream(IceCream),
            }
            #[derive(GraphQLEnum)]
            enum Fruit {
                Apple,
                Orange,
            }
            #[derive(GraphQLInputObject)]
            struct Coordinate {
                latitude: f64,
                longitude: f64,
            }
            struct Query;
            #[graphql_object]
            impl Query {
                fn blah() -> bool {
                    true
                }
                /// This is whatever's description.
                fn whatever() -> String {
                    "foo".into()
                }
                fn arr(stuff: Vec<Coordinate>) -> Option<&'static str> {
                    (!stuff.is_empty()).then_some("stuff")
                }
                fn fruit() -> Fruit {
                    Fruit::Apple
                }
                fn gluten_free(flavor: String) -> GlutenFree {
                    if flavor == "savory" {
                        GlutenFree::Cake(Cake::default())
                    } else {
                        GlutenFree::IceCream(IceCream::default())
                    }
                }
                #[deprecated]
                fn old() -> i32 {
                    42
                }
                #[deprecated(note = "This field is deprecated, use another.")]
                fn really_old() -> f64 {
                    42.0
                }
            }

            let schema = RootNode::new(
                Query,
                EmptyMutation::<()>::new(),
                EmptySubscription::<()>::new(),
            );
            let ast = graphql_parser::parse_schema::<&str>(
                r#"
                union GlutenFree = Cake | IceCream
                enum Fruit {
                    APPLE
                    ORANGE
                }
                type Cake {
                    fresh: Boolean!
                }
                type IceCream {
                    cold: Boolean!
                }
                type Query {
                  blah: Boolean!
                  "This is whatever's description."
                  whatever: String!
                  arr(stuff: [Coordinate!]!): String
                  fruit: Fruit!
                  glutenFree(flavor: String!): GlutenFree!
                  old: Int! @deprecated
                  reallyOld: Float! @deprecated(reason: "This field is deprecated, use another.")
                }
                input Coordinate {
                    latitude: Float!
                    longitude: Float!
                }
                schema {
                  query: Query
                }
            "#,
            )
            .unwrap();
            assert_eq!(ast.to_string(), schema.as_schema_language());
        }
    }
}
