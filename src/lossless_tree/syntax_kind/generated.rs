// Generated from tree-sitter-scala by the `syntax_kinds_are_up_to_date` test.
// Do not edit; regenerate with `UPDATE_GENERATED=1 cargo test syntax_kinds_are_up_to_date`.

/// The kind of a node or token. See the module docs for naming.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum SyntaxKind {
    /// `access_modifier`
    AccessModifier,
    /// `access_qualifier`
    AccessQualifier,
    /// `alternative_pattern`
    AlternativePattern,
    /// `annotated_type`
    AnnotatedType,
    /// `annotation`
    Annotation,
    /// `applied_constructor_type`
    AppliedConstructorType,
    /// `arguments`
    Arguments,
    /// `arrow_renamed_identifier`
    ArrowRenamedIdentifier,
    /// `as_renamed_identifier`
    AsRenamedIdentifier,
    /// `ascription_expression`
    AscriptionExpression,
    /// `assignment_expression`
    AssignmentExpression,
    /// `binding`
    Binding,
    /// `bindings`
    Bindings,
    /// `block`
    Block,
    /// `block_comment`
    BlockComment,
    /// `boolean_literal`
    BooleanLiteral,
    /// `call_expression`
    CallExpression,
    /// `capture_pattern`
    CapturePattern,
    /// `capture_ref`
    CaptureRef,
    /// `capture_set`
    CaptureSet,
    /// `capture_variable`
    CaptureVariable,
    /// `capturing_type`
    CapturingType,
    /// `case_block`
    CaseBlock,
    /// `case_class_pattern`
    CaseClassPattern,
    /// `case_clause`
    CaseClause,
    /// `catch_clause`
    CatchClause,
    /// `character_literal`
    CharacterLiteral,
    /// `class_definition`
    ClassDefinition,
    /// `class_parameter`
    ClassParameter,
    /// `class_parameters`
    ClassParameters,
    /// `colon_argument`
    ColonArgument,
    /// `comment`
    Comment,
    /// `compilation_unit`
    CompilationUnit,
    /// `compound_type`
    CompoundType,
    /// `consume_modifier`
    ConsumeModifier,
    /// `context_bound`
    ContextBound,
    /// `contravariant_type_parameter`
    ContravariantTypeParameter,
    /// `covariant_type_parameter`
    CovariantTypeParameter,
    /// `derives_clause`
    DerivesClause,
    /// `do_while_expression`
    DoWhileExpression,
    /// `early_defs`
    EarlyDefs,
    /// `end_marker`
    EndMarker,
    /// `enum_body`
    EnumBody,
    /// `enum_case_definitions`
    EnumCaseDefinitions,
    /// `enum_definition`
    EnumDefinition,
    /// `enumerator`
    Enumerator,
    /// `enumerators`
    Enumerators,
    /// `erased_modifier`
    ErasedModifier,
    /// `error_sentinel`
    ErrorSentinel,
    /// `escape_sequence`
    EscapeSequence,
    /// `existential_type`
    ExistentialType,
    /// `export_declaration`
    ExportDeclaration,
    /// `extends_clause`
    ExtendsClause,
    /// `extension_definition`
    ExtensionDefinition,
    /// `field_expression`
    FieldExpression,
    /// `finally_clause`
    FinallyClause,
    /// `floating_point_literal`
    FloatingPointLiteral,
    /// `for_expression`
    ForExpression,
    /// `full_enum_case`
    FullEnumCase,
    /// `function_declaration`
    FunctionDeclaration,
    /// `function_definition`
    FunctionDefinition,
    /// `function_type`
    FunctionType,
    /// `generic_function`
    GenericFunction,
    /// `generic_type`
    GenericType,
    /// `given_conditional`
    GivenConditional,
    /// `given_definition`
    GivenDefinition,
    /// `given_pattern`
    GivenPattern,
    /// `guard`
    Guard,
    /// `identifier`
    Identifier,
    /// `identifiers`
    Identifiers,
    /// `if_expression`
    IfExpression,
    /// `import_declaration`
    ImportDeclaration,
    /// `indented_block`
    IndentedBlock,
    /// `indented_cases`
    IndentedCases,
    /// `infix_expression`
    InfixExpression,
    /// `infix_modifier`
    InfixModifier,
    /// `infix_pattern`
    InfixPattern,
    /// `infix_type`
    InfixType,
    /// `inline_modifier`
    InlineModifier,
    /// `instance_expression`
    InstanceExpression,
    /// `integer_literal`
    IntegerLiteral,
    /// `interpolated_string`
    InterpolatedString,
    /// `interpolated_string_expression`
    InterpolatedStringExpression,
    /// `interpolation`
    Interpolation,
    /// `into_modifier`
    IntoModifier,
    /// `lambda_expression`
    LambdaExpression,
    /// `lazy_parameter_type`
    LazyParameterType,
    /// `literal_type`
    LiteralType,
    /// `lower_bound`
    LowerBound,
    /// `macro_body`
    MacroBody,
    /// `match_expression`
    MatchExpression,
    /// `match_type`
    MatchType,
    /// `method_value`
    MethodValue,
    /// `modifiers`
    Modifiers,
    /// `name_and_type`
    NameAndType,
    /// `named_pattern`
    NamedPattern,
    /// `named_tuple_pattern`
    NamedTuplePattern,
    /// `named_tuple_type`
    NamedTupleType,
    /// `named_type_argument`
    NamedTypeArgument,
    /// `namespace_selectors`
    NamespaceSelectors,
    /// `namespace_wildcard`
    NamespaceWildcard,
    /// `null_literal`
    NullLiteral,
    /// `object_definition`
    ObjectDefinition,
    /// `opaque_modifier`
    OpaqueModifier,
    /// `open_modifier`
    OpenModifier,
    /// `operator_identifier`
    OperatorIdentifier,
    /// `package_clause`
    PackageClause,
    /// `package_identifier`
    PackageIdentifier,
    /// `package_object`
    PackageObject,
    /// `parameter`
    Parameter,
    /// `parameter_types`
    ParameterTypes,
    /// `parameters`
    Parameters,
    /// `parenthesized_expression`
    ParenthesizedExpression,
    /// `postfix_expression`
    PostfixExpression,
    /// `prefix_expression`
    PrefixExpression,
    /// `projected_type`
    ProjectedType,
    /// `quote_expression`
    QuoteExpression,
    /// `refinement`
    Refinement,
    /// `repeat_pattern`
    RepeatPattern,
    /// `repeated_parameter_type`
    RepeatedParameterType,
    /// `return_expression`
    ReturnExpression,
    /// `self_type`
    SelfType,
    /// `simple_enum_case`
    SimpleEnumCase,
    /// `singleton_type`
    SingletonType,
    /// `splice_expression`
    SpliceExpression,
    /// `stable_identifier`
    StableIdentifier,
    /// `stable_type_identifier`
    StableTypeIdentifier,
    /// `string`
    String,
    /// `structural_type`
    StructuralType,
    /// `template_body`
    TemplateBody,
    /// `throw_expression`
    ThrowExpression,
    /// `tracked_modifier`
    TrackedModifier,
    /// `trait_definition`
    TraitDefinition,
    /// `transparent_modifier`
    TransparentModifier,
    /// `try_expression`
    TryExpression,
    /// `tuple_expression`
    TupleExpression,
    /// `tuple_pattern`
    TuplePattern,
    /// `tuple_type`
    TupleType,
    /// `type_arguments`
    TypeArguments,
    /// `type_case_clause`
    TypeCaseClause,
    /// `type_definition`
    TypeDefinition,
    /// `type_identifier`
    TypeIdentifier,
    /// `type_lambda`
    TypeLambda,
    /// `type_parameters`
    TypeParameters,
    /// `typed_pattern`
    TypedPattern,
    /// `unit`
    Unit,
    /// `update_modifier`
    UpdateModifier,
    /// `upper_bound`
    UpperBound,
    /// `uses_clause`
    UsesClause,
    /// `using_directive`
    UsingDirective,
    /// `using_directive_key`
    UsingDirectiveKey,
    /// `using_directive_value`
    UsingDirectiveValue,
    /// `val_declaration`
    ValDeclaration,
    /// `val_definition`
    ValDefinition,
    /// `var_declaration`
    VarDeclaration,
    /// `var_definition`
    VarDefinition,
    /// `vararg`
    Vararg,
    /// `view_bound`
    ViewBound,
    /// `while_expression`
    WhileExpression,
    /// `wildcard`
    Wildcard,
    /// `with_template_body`
    WithTemplateBody,
    /// `xml_attribute`
    XmlAttribute,
    /// `xml_cdata`
    XmlCdata,
    /// `xml_comment`
    XmlComment,
    /// `xml_element`
    XmlElement,
    /// `xml_expression`
    XmlExpression,
    /// `xml_name`
    XmlName,
    /// `xml_pattern`
    XmlPattern,
    /// `xml_processing_instruction`
    XmlProcessingInstruction,
    /// `xml_string`
    XmlString,
    /// `xml_text`
    XmlText,
    /// `"!"`
    Bang,
    /// `"\""`
    Quote,
    /// `"\"\"\""`
    QuoteQuoteQuote,
    /// `"#"`
    Pound,
    /// `"$"`
    Dollar,
    /// `"$["`
    DollarLBrack,
    /// `"${"`
    DollarLCurly,
    /// `"'"`
    Apos,
    /// `"("`
    LParen,
    /// `")"`
    RParen,
    /// `"*"`
    Star,
    /// `"+"`
    Plus,
    /// `","`
    Comma,
    /// `"-"`
    Minus,
    /// `"->"`
    MinusGt,
    /// `"."`
    Dot,
    /// `"//"`
    SlashSlash,
    /// `"/>"`
    SlashGt,
    /// `":"`
    Colon,
    /// `";"`
    Semi,
    /// `"<"`
    Lt,
    /// `"<%"`
    LtPercent,
    /// `"<-"`
    LtMinus,
    /// `"</"`
    LtSlash,
    /// `"<:"`
    LtColon,
    /// `"="`
    Eq,
    /// `"=>"`
    EqGt,
    /// `"=>>"`
    EqGtGt,
    /// `">"`
    Gt,
    /// `">:"`
    GtColon,
    /// `"?->"`
    QuestionMinusGt,
    /// `"?=>"`
    QuestionEqGt,
    /// `"@"`
    At,
    /// `"["`
    LBrack,
    /// `"]"`
    RBrack,
    /// `"^"`
    Caret,
    /// `"_"`
    Underscore,
    /// `"_end_ident"`
    EndIdentKw,
    /// `"abstract"`
    AbstractKw,
    /// `"as"`
    AsKw,
    /// `"cap"`
    CapKw,
    /// `"case"`
    CaseKw,
    /// `"catch"`
    CatchKw,
    /// `"class"`
    ClassKw,
    /// `"def"`
    DefKw,
    /// `"derives"`
    DerivesKw,
    /// `"do"`
    DoKw,
    /// `"else"`
    ElseKw,
    /// `"enum"`
    EnumKw,
    /// `"except"`
    ExceptKw,
    /// `"export"`
    ExportKw,
    /// `"extends"`
    ExtendsKw,
    /// `"extension"`
    ExtensionKw,
    /// `"false"`
    FalseKw,
    /// `"final"`
    FinalKw,
    /// `"finally"`
    FinallyKw,
    /// `"for"`
    ForKw,
    /// `"forSome"`
    ForSomeKw,
    /// `"given"`
    GivenKw,
    /// `"if"`
    IfKw,
    /// `"implicit"`
    ImplicitKw,
    /// `"import"`
    ImportKw,
    /// `"lazy"`
    LazyKw,
    /// `"macro"`
    MacroKw,
    /// `"match"`
    MatchKw,
    /// `"new"`
    NewKw,
    /// `"object"`
    ObjectKw,
    /// `"only"`
    OnlyKw,
    /// `"override"`
    OverrideKw,
    /// `"package"`
    PackageKw,
    /// `"private"`
    PrivateKw,
    /// `"protected"`
    ProtectedKw,
    /// `"rd"`
    RdKw,
    /// `"return"`
    ReturnKw,
    /// `"sealed"`
    SealedKw,
    /// `"super"`
    SuperKw,
    /// `"then"`
    ThenKw,
    /// `"this"`
    ThisKw,
    /// `"throw"`
    ThrowKw,
    /// `"trait"`
    TraitKw,
    /// `"true"`
    TrueKw,
    /// `"try"`
    TryKw,
    /// `"type"`
    TypeKw,
    /// `"uses"`
    UsesKw,
    /// `"using"`
    UsingKw,
    /// `"val"`
    ValKw,
    /// `"var"`
    VarKw,
    /// `"while"`
    WhileKw,
    /// `"with"`
    WithKw,
    /// `"yield"`
    YieldKw,
    /// `"{"`
    LCurly,
    /// `"|"`
    Pipe,
    /// `"}"`
    RCurly,
    /// `"~"`
    Tilde,
    /// `"←"`
    UnicodeLeftArrow,
    /// tree-sitter's error node: input it couldn't parse.
    Error,
    /// Whitespace between tokens.
    Whitespace,
    /// Text inside a node that isn't in any child, such as the literal
    /// parts of an interpolated string.
    Text,
}

impl SyntaxKind {
    /// The grammar's name for this kind, such as `class_definition` or `{`.
    pub fn name(self) -> &'static str {
        match self {
            SyntaxKind::AccessModifier => "access_modifier",
            SyntaxKind::AccessQualifier => "access_qualifier",
            SyntaxKind::AlternativePattern => "alternative_pattern",
            SyntaxKind::AnnotatedType => "annotated_type",
            SyntaxKind::Annotation => "annotation",
            SyntaxKind::AppliedConstructorType => "applied_constructor_type",
            SyntaxKind::Arguments => "arguments",
            SyntaxKind::ArrowRenamedIdentifier => "arrow_renamed_identifier",
            SyntaxKind::AsRenamedIdentifier => "as_renamed_identifier",
            SyntaxKind::AscriptionExpression => "ascription_expression",
            SyntaxKind::AssignmentExpression => "assignment_expression",
            SyntaxKind::Binding => "binding",
            SyntaxKind::Bindings => "bindings",
            SyntaxKind::Block => "block",
            SyntaxKind::BlockComment => "block_comment",
            SyntaxKind::BooleanLiteral => "boolean_literal",
            SyntaxKind::CallExpression => "call_expression",
            SyntaxKind::CapturePattern => "capture_pattern",
            SyntaxKind::CaptureRef => "capture_ref",
            SyntaxKind::CaptureSet => "capture_set",
            SyntaxKind::CaptureVariable => "capture_variable",
            SyntaxKind::CapturingType => "capturing_type",
            SyntaxKind::CaseBlock => "case_block",
            SyntaxKind::CaseClassPattern => "case_class_pattern",
            SyntaxKind::CaseClause => "case_clause",
            SyntaxKind::CatchClause => "catch_clause",
            SyntaxKind::CharacterLiteral => "character_literal",
            SyntaxKind::ClassDefinition => "class_definition",
            SyntaxKind::ClassParameter => "class_parameter",
            SyntaxKind::ClassParameters => "class_parameters",
            SyntaxKind::ColonArgument => "colon_argument",
            SyntaxKind::Comment => "comment",
            SyntaxKind::CompilationUnit => "compilation_unit",
            SyntaxKind::CompoundType => "compound_type",
            SyntaxKind::ConsumeModifier => "consume_modifier",
            SyntaxKind::ContextBound => "context_bound",
            SyntaxKind::ContravariantTypeParameter => "contravariant_type_parameter",
            SyntaxKind::CovariantTypeParameter => "covariant_type_parameter",
            SyntaxKind::DerivesClause => "derives_clause",
            SyntaxKind::DoWhileExpression => "do_while_expression",
            SyntaxKind::EarlyDefs => "early_defs",
            SyntaxKind::EndMarker => "end_marker",
            SyntaxKind::EnumBody => "enum_body",
            SyntaxKind::EnumCaseDefinitions => "enum_case_definitions",
            SyntaxKind::EnumDefinition => "enum_definition",
            SyntaxKind::Enumerator => "enumerator",
            SyntaxKind::Enumerators => "enumerators",
            SyntaxKind::ErasedModifier => "erased_modifier",
            SyntaxKind::ErrorSentinel => "error_sentinel",
            SyntaxKind::EscapeSequence => "escape_sequence",
            SyntaxKind::ExistentialType => "existential_type",
            SyntaxKind::ExportDeclaration => "export_declaration",
            SyntaxKind::ExtendsClause => "extends_clause",
            SyntaxKind::ExtensionDefinition => "extension_definition",
            SyntaxKind::FieldExpression => "field_expression",
            SyntaxKind::FinallyClause => "finally_clause",
            SyntaxKind::FloatingPointLiteral => "floating_point_literal",
            SyntaxKind::ForExpression => "for_expression",
            SyntaxKind::FullEnumCase => "full_enum_case",
            SyntaxKind::FunctionDeclaration => "function_declaration",
            SyntaxKind::FunctionDefinition => "function_definition",
            SyntaxKind::FunctionType => "function_type",
            SyntaxKind::GenericFunction => "generic_function",
            SyntaxKind::GenericType => "generic_type",
            SyntaxKind::GivenConditional => "given_conditional",
            SyntaxKind::GivenDefinition => "given_definition",
            SyntaxKind::GivenPattern => "given_pattern",
            SyntaxKind::Guard => "guard",
            SyntaxKind::Identifier => "identifier",
            SyntaxKind::Identifiers => "identifiers",
            SyntaxKind::IfExpression => "if_expression",
            SyntaxKind::ImportDeclaration => "import_declaration",
            SyntaxKind::IndentedBlock => "indented_block",
            SyntaxKind::IndentedCases => "indented_cases",
            SyntaxKind::InfixExpression => "infix_expression",
            SyntaxKind::InfixModifier => "infix_modifier",
            SyntaxKind::InfixPattern => "infix_pattern",
            SyntaxKind::InfixType => "infix_type",
            SyntaxKind::InlineModifier => "inline_modifier",
            SyntaxKind::InstanceExpression => "instance_expression",
            SyntaxKind::IntegerLiteral => "integer_literal",
            SyntaxKind::InterpolatedString => "interpolated_string",
            SyntaxKind::InterpolatedStringExpression => "interpolated_string_expression",
            SyntaxKind::Interpolation => "interpolation",
            SyntaxKind::IntoModifier => "into_modifier",
            SyntaxKind::LambdaExpression => "lambda_expression",
            SyntaxKind::LazyParameterType => "lazy_parameter_type",
            SyntaxKind::LiteralType => "literal_type",
            SyntaxKind::LowerBound => "lower_bound",
            SyntaxKind::MacroBody => "macro_body",
            SyntaxKind::MatchExpression => "match_expression",
            SyntaxKind::MatchType => "match_type",
            SyntaxKind::MethodValue => "method_value",
            SyntaxKind::Modifiers => "modifiers",
            SyntaxKind::NameAndType => "name_and_type",
            SyntaxKind::NamedPattern => "named_pattern",
            SyntaxKind::NamedTuplePattern => "named_tuple_pattern",
            SyntaxKind::NamedTupleType => "named_tuple_type",
            SyntaxKind::NamedTypeArgument => "named_type_argument",
            SyntaxKind::NamespaceSelectors => "namespace_selectors",
            SyntaxKind::NamespaceWildcard => "namespace_wildcard",
            SyntaxKind::NullLiteral => "null_literal",
            SyntaxKind::ObjectDefinition => "object_definition",
            SyntaxKind::OpaqueModifier => "opaque_modifier",
            SyntaxKind::OpenModifier => "open_modifier",
            SyntaxKind::OperatorIdentifier => "operator_identifier",
            SyntaxKind::PackageClause => "package_clause",
            SyntaxKind::PackageIdentifier => "package_identifier",
            SyntaxKind::PackageObject => "package_object",
            SyntaxKind::Parameter => "parameter",
            SyntaxKind::ParameterTypes => "parameter_types",
            SyntaxKind::Parameters => "parameters",
            SyntaxKind::ParenthesizedExpression => "parenthesized_expression",
            SyntaxKind::PostfixExpression => "postfix_expression",
            SyntaxKind::PrefixExpression => "prefix_expression",
            SyntaxKind::ProjectedType => "projected_type",
            SyntaxKind::QuoteExpression => "quote_expression",
            SyntaxKind::Refinement => "refinement",
            SyntaxKind::RepeatPattern => "repeat_pattern",
            SyntaxKind::RepeatedParameterType => "repeated_parameter_type",
            SyntaxKind::ReturnExpression => "return_expression",
            SyntaxKind::SelfType => "self_type",
            SyntaxKind::SimpleEnumCase => "simple_enum_case",
            SyntaxKind::SingletonType => "singleton_type",
            SyntaxKind::SpliceExpression => "splice_expression",
            SyntaxKind::StableIdentifier => "stable_identifier",
            SyntaxKind::StableTypeIdentifier => "stable_type_identifier",
            SyntaxKind::String => "string",
            SyntaxKind::StructuralType => "structural_type",
            SyntaxKind::TemplateBody => "template_body",
            SyntaxKind::ThrowExpression => "throw_expression",
            SyntaxKind::TrackedModifier => "tracked_modifier",
            SyntaxKind::TraitDefinition => "trait_definition",
            SyntaxKind::TransparentModifier => "transparent_modifier",
            SyntaxKind::TryExpression => "try_expression",
            SyntaxKind::TupleExpression => "tuple_expression",
            SyntaxKind::TuplePattern => "tuple_pattern",
            SyntaxKind::TupleType => "tuple_type",
            SyntaxKind::TypeArguments => "type_arguments",
            SyntaxKind::TypeCaseClause => "type_case_clause",
            SyntaxKind::TypeDefinition => "type_definition",
            SyntaxKind::TypeIdentifier => "type_identifier",
            SyntaxKind::TypeLambda => "type_lambda",
            SyntaxKind::TypeParameters => "type_parameters",
            SyntaxKind::TypedPattern => "typed_pattern",
            SyntaxKind::Unit => "unit",
            SyntaxKind::UpdateModifier => "update_modifier",
            SyntaxKind::UpperBound => "upper_bound",
            SyntaxKind::UsesClause => "uses_clause",
            SyntaxKind::UsingDirective => "using_directive",
            SyntaxKind::UsingDirectiveKey => "using_directive_key",
            SyntaxKind::UsingDirectiveValue => "using_directive_value",
            SyntaxKind::ValDeclaration => "val_declaration",
            SyntaxKind::ValDefinition => "val_definition",
            SyntaxKind::VarDeclaration => "var_declaration",
            SyntaxKind::VarDefinition => "var_definition",
            SyntaxKind::Vararg => "vararg",
            SyntaxKind::ViewBound => "view_bound",
            SyntaxKind::WhileExpression => "while_expression",
            SyntaxKind::Wildcard => "wildcard",
            SyntaxKind::WithTemplateBody => "with_template_body",
            SyntaxKind::XmlAttribute => "xml_attribute",
            SyntaxKind::XmlCdata => "xml_cdata",
            SyntaxKind::XmlComment => "xml_comment",
            SyntaxKind::XmlElement => "xml_element",
            SyntaxKind::XmlExpression => "xml_expression",
            SyntaxKind::XmlName => "xml_name",
            SyntaxKind::XmlPattern => "xml_pattern",
            SyntaxKind::XmlProcessingInstruction => "xml_processing_instruction",
            SyntaxKind::XmlString => "xml_string",
            SyntaxKind::XmlText => "xml_text",
            SyntaxKind::Bang => "!",
            SyntaxKind::Quote => "\"",
            SyntaxKind::QuoteQuoteQuote => "\"\"\"",
            SyntaxKind::Pound => "#",
            SyntaxKind::Dollar => "$",
            SyntaxKind::DollarLBrack => "$[",
            SyntaxKind::DollarLCurly => "${",
            SyntaxKind::Apos => "'",
            SyntaxKind::LParen => "(",
            SyntaxKind::RParen => ")",
            SyntaxKind::Star => "*",
            SyntaxKind::Plus => "+",
            SyntaxKind::Comma => ",",
            SyntaxKind::Minus => "-",
            SyntaxKind::MinusGt => "->",
            SyntaxKind::Dot => ".",
            SyntaxKind::SlashSlash => "//",
            SyntaxKind::SlashGt => "/>",
            SyntaxKind::Colon => ":",
            SyntaxKind::Semi => ";",
            SyntaxKind::Lt => "<",
            SyntaxKind::LtPercent => "<%",
            SyntaxKind::LtMinus => "<-",
            SyntaxKind::LtSlash => "</",
            SyntaxKind::LtColon => "<:",
            SyntaxKind::Eq => "=",
            SyntaxKind::EqGt => "=>",
            SyntaxKind::EqGtGt => "=>>",
            SyntaxKind::Gt => ">",
            SyntaxKind::GtColon => ">:",
            SyntaxKind::QuestionMinusGt => "?->",
            SyntaxKind::QuestionEqGt => "?=>",
            SyntaxKind::At => "@",
            SyntaxKind::LBrack => "[",
            SyntaxKind::RBrack => "]",
            SyntaxKind::Caret => "^",
            SyntaxKind::Underscore => "_",
            SyntaxKind::EndIdentKw => "_end_ident",
            SyntaxKind::AbstractKw => "abstract",
            SyntaxKind::AsKw => "as",
            SyntaxKind::CapKw => "cap",
            SyntaxKind::CaseKw => "case",
            SyntaxKind::CatchKw => "catch",
            SyntaxKind::ClassKw => "class",
            SyntaxKind::DefKw => "def",
            SyntaxKind::DerivesKw => "derives",
            SyntaxKind::DoKw => "do",
            SyntaxKind::ElseKw => "else",
            SyntaxKind::EnumKw => "enum",
            SyntaxKind::ExceptKw => "except",
            SyntaxKind::ExportKw => "export",
            SyntaxKind::ExtendsKw => "extends",
            SyntaxKind::ExtensionKw => "extension",
            SyntaxKind::FalseKw => "false",
            SyntaxKind::FinalKw => "final",
            SyntaxKind::FinallyKw => "finally",
            SyntaxKind::ForKw => "for",
            SyntaxKind::ForSomeKw => "forSome",
            SyntaxKind::GivenKw => "given",
            SyntaxKind::IfKw => "if",
            SyntaxKind::ImplicitKw => "implicit",
            SyntaxKind::ImportKw => "import",
            SyntaxKind::LazyKw => "lazy",
            SyntaxKind::MacroKw => "macro",
            SyntaxKind::MatchKw => "match",
            SyntaxKind::NewKw => "new",
            SyntaxKind::ObjectKw => "object",
            SyntaxKind::OnlyKw => "only",
            SyntaxKind::OverrideKw => "override",
            SyntaxKind::PackageKw => "package",
            SyntaxKind::PrivateKw => "private",
            SyntaxKind::ProtectedKw => "protected",
            SyntaxKind::RdKw => "rd",
            SyntaxKind::ReturnKw => "return",
            SyntaxKind::SealedKw => "sealed",
            SyntaxKind::SuperKw => "super",
            SyntaxKind::ThenKw => "then",
            SyntaxKind::ThisKw => "this",
            SyntaxKind::ThrowKw => "throw",
            SyntaxKind::TraitKw => "trait",
            SyntaxKind::TrueKw => "true",
            SyntaxKind::TryKw => "try",
            SyntaxKind::TypeKw => "type",
            SyntaxKind::UsesKw => "uses",
            SyntaxKind::UsingKw => "using",
            SyntaxKind::ValKw => "val",
            SyntaxKind::VarKw => "var",
            SyntaxKind::WhileKw => "while",
            SyntaxKind::WithKw => "with",
            SyntaxKind::YieldKw => "yield",
            SyntaxKind::LCurly => "{",
            SyntaxKind::Pipe => "|",
            SyntaxKind::RCurly => "}",
            SyntaxKind::Tilde => "~",
            SyntaxKind::UnicodeLeftArrow => "←",
            SyntaxKind::Error => "ERROR",
            SyntaxKind::Whitespace => "whitespace",
            SyntaxKind::Text => "text",
        }
    }

    /// Named kinds (`identifier`, `class_definition`) versus keywords and
    /// punctuation (`"class"`, `"{"`).
    pub fn is_named(self) -> bool {
        !matches!(
            self,
            SyntaxKind::Bang
                | SyntaxKind::Quote
                | SyntaxKind::QuoteQuoteQuote
                | SyntaxKind::Pound
                | SyntaxKind::Dollar
                | SyntaxKind::DollarLBrack
                | SyntaxKind::DollarLCurly
                | SyntaxKind::Apos
                | SyntaxKind::LParen
                | SyntaxKind::RParen
                | SyntaxKind::Star
                | SyntaxKind::Plus
                | SyntaxKind::Comma
                | SyntaxKind::Minus
                | SyntaxKind::MinusGt
                | SyntaxKind::Dot
                | SyntaxKind::SlashSlash
                | SyntaxKind::SlashGt
                | SyntaxKind::Colon
                | SyntaxKind::Semi
                | SyntaxKind::Lt
                | SyntaxKind::LtPercent
                | SyntaxKind::LtMinus
                | SyntaxKind::LtSlash
                | SyntaxKind::LtColon
                | SyntaxKind::Eq
                | SyntaxKind::EqGt
                | SyntaxKind::EqGtGt
                | SyntaxKind::Gt
                | SyntaxKind::GtColon
                | SyntaxKind::QuestionMinusGt
                | SyntaxKind::QuestionEqGt
                | SyntaxKind::At
                | SyntaxKind::LBrack
                | SyntaxKind::RBrack
                | SyntaxKind::Caret
                | SyntaxKind::Underscore
                | SyntaxKind::EndIdentKw
                | SyntaxKind::AbstractKw
                | SyntaxKind::AsKw
                | SyntaxKind::CapKw
                | SyntaxKind::CaseKw
                | SyntaxKind::CatchKw
                | SyntaxKind::ClassKw
                | SyntaxKind::DefKw
                | SyntaxKind::DerivesKw
                | SyntaxKind::DoKw
                | SyntaxKind::ElseKw
                | SyntaxKind::EnumKw
                | SyntaxKind::ExceptKw
                | SyntaxKind::ExportKw
                | SyntaxKind::ExtendsKw
                | SyntaxKind::ExtensionKw
                | SyntaxKind::FalseKw
                | SyntaxKind::FinalKw
                | SyntaxKind::FinallyKw
                | SyntaxKind::ForKw
                | SyntaxKind::ForSomeKw
                | SyntaxKind::GivenKw
                | SyntaxKind::IfKw
                | SyntaxKind::ImplicitKw
                | SyntaxKind::ImportKw
                | SyntaxKind::LazyKw
                | SyntaxKind::MacroKw
                | SyntaxKind::MatchKw
                | SyntaxKind::NewKw
                | SyntaxKind::ObjectKw
                | SyntaxKind::OnlyKw
                | SyntaxKind::OverrideKw
                | SyntaxKind::PackageKw
                | SyntaxKind::PrivateKw
                | SyntaxKind::ProtectedKw
                | SyntaxKind::RdKw
                | SyntaxKind::ReturnKw
                | SyntaxKind::SealedKw
                | SyntaxKind::SuperKw
                | SyntaxKind::ThenKw
                | SyntaxKind::ThisKw
                | SyntaxKind::ThrowKw
                | SyntaxKind::TraitKw
                | SyntaxKind::TrueKw
                | SyntaxKind::TryKw
                | SyntaxKind::TypeKw
                | SyntaxKind::UsesKw
                | SyntaxKind::UsingKw
                | SyntaxKind::ValKw
                | SyntaxKind::VarKw
                | SyntaxKind::WhileKw
                | SyntaxKind::WithKw
                | SyntaxKind::YieldKw
                | SyntaxKind::LCurly
                | SyntaxKind::Pipe
                | SyntaxKind::RCurly
                | SyntaxKind::Tilde
                | SyntaxKind::UnicodeLeftArrow
        )
    }

    /// Word-like keywords such as `class`, `match` and `given`.
    pub fn is_keyword(self) -> bool {
        matches!(
            self,
            SyntaxKind::EndIdentKw
                | SyntaxKind::AbstractKw
                | SyntaxKind::AsKw
                | SyntaxKind::CapKw
                | SyntaxKind::CaseKw
                | SyntaxKind::CatchKw
                | SyntaxKind::ClassKw
                | SyntaxKind::DefKw
                | SyntaxKind::DerivesKw
                | SyntaxKind::DoKw
                | SyntaxKind::ElseKw
                | SyntaxKind::EnumKw
                | SyntaxKind::ExceptKw
                | SyntaxKind::ExportKw
                | SyntaxKind::ExtendsKw
                | SyntaxKind::ExtensionKw
                | SyntaxKind::FalseKw
                | SyntaxKind::FinalKw
                | SyntaxKind::FinallyKw
                | SyntaxKind::ForKw
                | SyntaxKind::ForSomeKw
                | SyntaxKind::GivenKw
                | SyntaxKind::IfKw
                | SyntaxKind::ImplicitKw
                | SyntaxKind::ImportKw
                | SyntaxKind::LazyKw
                | SyntaxKind::MacroKw
                | SyntaxKind::MatchKw
                | SyntaxKind::NewKw
                | SyntaxKind::ObjectKw
                | SyntaxKind::OnlyKw
                | SyntaxKind::OverrideKw
                | SyntaxKind::PackageKw
                | SyntaxKind::PrivateKw
                | SyntaxKind::ProtectedKw
                | SyntaxKind::RdKw
                | SyntaxKind::ReturnKw
                | SyntaxKind::SealedKw
                | SyntaxKind::SuperKw
                | SyntaxKind::ThenKw
                | SyntaxKind::ThisKw
                | SyntaxKind::ThrowKw
                | SyntaxKind::TraitKw
                | SyntaxKind::TrueKw
                | SyntaxKind::TryKw
                | SyntaxKind::TypeKw
                | SyntaxKind::UsesKw
                | SyntaxKind::UsingKw
                | SyntaxKind::ValKw
                | SyntaxKind::VarKw
                | SyntaxKind::WhileKw
                | SyntaxKind::WithKw
                | SyntaxKind::YieldKw
        )
    }

    /// Looks up a grammar kind by name.
    pub(crate) fn from_grammar(name: &str, named: bool) -> Option<SyntaxKind> {
        Some(match (name, named) {
            ("access_modifier", true) => SyntaxKind::AccessModifier,
            ("access_qualifier", true) => SyntaxKind::AccessQualifier,
            ("alternative_pattern", true) => SyntaxKind::AlternativePattern,
            ("annotated_type", true) => SyntaxKind::AnnotatedType,
            ("annotation", true) => SyntaxKind::Annotation,
            ("applied_constructor_type", true) => SyntaxKind::AppliedConstructorType,
            ("arguments", true) => SyntaxKind::Arguments,
            ("arrow_renamed_identifier", true) => SyntaxKind::ArrowRenamedIdentifier,
            ("as_renamed_identifier", true) => SyntaxKind::AsRenamedIdentifier,
            ("ascription_expression", true) => SyntaxKind::AscriptionExpression,
            ("assignment_expression", true) => SyntaxKind::AssignmentExpression,
            ("binding", true) => SyntaxKind::Binding,
            ("bindings", true) => SyntaxKind::Bindings,
            ("block", true) => SyntaxKind::Block,
            ("block_comment", true) => SyntaxKind::BlockComment,
            ("boolean_literal", true) => SyntaxKind::BooleanLiteral,
            ("call_expression", true) => SyntaxKind::CallExpression,
            ("capture_pattern", true) => SyntaxKind::CapturePattern,
            ("capture_ref", true) => SyntaxKind::CaptureRef,
            ("capture_set", true) => SyntaxKind::CaptureSet,
            ("capture_variable", true) => SyntaxKind::CaptureVariable,
            ("capturing_type", true) => SyntaxKind::CapturingType,
            ("case_block", true) => SyntaxKind::CaseBlock,
            ("case_class_pattern", true) => SyntaxKind::CaseClassPattern,
            ("case_clause", true) => SyntaxKind::CaseClause,
            ("catch_clause", true) => SyntaxKind::CatchClause,
            ("character_literal", true) => SyntaxKind::CharacterLiteral,
            ("class_definition", true) => SyntaxKind::ClassDefinition,
            ("class_parameter", true) => SyntaxKind::ClassParameter,
            ("class_parameters", true) => SyntaxKind::ClassParameters,
            ("colon_argument", true) => SyntaxKind::ColonArgument,
            ("comment", true) => SyntaxKind::Comment,
            ("compilation_unit", true) => SyntaxKind::CompilationUnit,
            ("compound_type", true) => SyntaxKind::CompoundType,
            ("consume_modifier", true) => SyntaxKind::ConsumeModifier,
            ("context_bound", true) => SyntaxKind::ContextBound,
            ("contravariant_type_parameter", true) => SyntaxKind::ContravariantTypeParameter,
            ("covariant_type_parameter", true) => SyntaxKind::CovariantTypeParameter,
            ("derives_clause", true) => SyntaxKind::DerivesClause,
            ("do_while_expression", true) => SyntaxKind::DoWhileExpression,
            ("early_defs", true) => SyntaxKind::EarlyDefs,
            ("end_marker", true) => SyntaxKind::EndMarker,
            ("enum_body", true) => SyntaxKind::EnumBody,
            ("enum_case_definitions", true) => SyntaxKind::EnumCaseDefinitions,
            ("enum_definition", true) => SyntaxKind::EnumDefinition,
            ("enumerator", true) => SyntaxKind::Enumerator,
            ("enumerators", true) => SyntaxKind::Enumerators,
            ("erased_modifier", true) => SyntaxKind::ErasedModifier,
            ("error_sentinel", true) => SyntaxKind::ErrorSentinel,
            ("escape_sequence", true) => SyntaxKind::EscapeSequence,
            ("existential_type", true) => SyntaxKind::ExistentialType,
            ("export_declaration", true) => SyntaxKind::ExportDeclaration,
            ("extends_clause", true) => SyntaxKind::ExtendsClause,
            ("extension_definition", true) => SyntaxKind::ExtensionDefinition,
            ("field_expression", true) => SyntaxKind::FieldExpression,
            ("finally_clause", true) => SyntaxKind::FinallyClause,
            ("floating_point_literal", true) => SyntaxKind::FloatingPointLiteral,
            ("for_expression", true) => SyntaxKind::ForExpression,
            ("full_enum_case", true) => SyntaxKind::FullEnumCase,
            ("function_declaration", true) => SyntaxKind::FunctionDeclaration,
            ("function_definition", true) => SyntaxKind::FunctionDefinition,
            ("function_type", true) => SyntaxKind::FunctionType,
            ("generic_function", true) => SyntaxKind::GenericFunction,
            ("generic_type", true) => SyntaxKind::GenericType,
            ("given_conditional", true) => SyntaxKind::GivenConditional,
            ("given_definition", true) => SyntaxKind::GivenDefinition,
            ("given_pattern", true) => SyntaxKind::GivenPattern,
            ("guard", true) => SyntaxKind::Guard,
            ("identifier", true) => SyntaxKind::Identifier,
            ("identifiers", true) => SyntaxKind::Identifiers,
            ("if_expression", true) => SyntaxKind::IfExpression,
            ("import_declaration", true) => SyntaxKind::ImportDeclaration,
            ("indented_block", true) => SyntaxKind::IndentedBlock,
            ("indented_cases", true) => SyntaxKind::IndentedCases,
            ("infix_expression", true) => SyntaxKind::InfixExpression,
            ("infix_modifier", true) => SyntaxKind::InfixModifier,
            ("infix_pattern", true) => SyntaxKind::InfixPattern,
            ("infix_type", true) => SyntaxKind::InfixType,
            ("inline_modifier", true) => SyntaxKind::InlineModifier,
            ("instance_expression", true) => SyntaxKind::InstanceExpression,
            ("integer_literal", true) => SyntaxKind::IntegerLiteral,
            ("interpolated_string", true) => SyntaxKind::InterpolatedString,
            ("interpolated_string_expression", true) => SyntaxKind::InterpolatedStringExpression,
            ("interpolation", true) => SyntaxKind::Interpolation,
            ("into_modifier", true) => SyntaxKind::IntoModifier,
            ("lambda_expression", true) => SyntaxKind::LambdaExpression,
            ("lazy_parameter_type", true) => SyntaxKind::LazyParameterType,
            ("literal_type", true) => SyntaxKind::LiteralType,
            ("lower_bound", true) => SyntaxKind::LowerBound,
            ("macro_body", true) => SyntaxKind::MacroBody,
            ("match_expression", true) => SyntaxKind::MatchExpression,
            ("match_type", true) => SyntaxKind::MatchType,
            ("method_value", true) => SyntaxKind::MethodValue,
            ("modifiers", true) => SyntaxKind::Modifiers,
            ("name_and_type", true) => SyntaxKind::NameAndType,
            ("named_pattern", true) => SyntaxKind::NamedPattern,
            ("named_tuple_pattern", true) => SyntaxKind::NamedTuplePattern,
            ("named_tuple_type", true) => SyntaxKind::NamedTupleType,
            ("named_type_argument", true) => SyntaxKind::NamedTypeArgument,
            ("namespace_selectors", true) => SyntaxKind::NamespaceSelectors,
            ("namespace_wildcard", true) => SyntaxKind::NamespaceWildcard,
            ("null_literal", true) => SyntaxKind::NullLiteral,
            ("object_definition", true) => SyntaxKind::ObjectDefinition,
            ("opaque_modifier", true) => SyntaxKind::OpaqueModifier,
            ("open_modifier", true) => SyntaxKind::OpenModifier,
            ("operator_identifier", true) => SyntaxKind::OperatorIdentifier,
            ("package_clause", true) => SyntaxKind::PackageClause,
            ("package_identifier", true) => SyntaxKind::PackageIdentifier,
            ("package_object", true) => SyntaxKind::PackageObject,
            ("parameter", true) => SyntaxKind::Parameter,
            ("parameter_types", true) => SyntaxKind::ParameterTypes,
            ("parameters", true) => SyntaxKind::Parameters,
            ("parenthesized_expression", true) => SyntaxKind::ParenthesizedExpression,
            ("postfix_expression", true) => SyntaxKind::PostfixExpression,
            ("prefix_expression", true) => SyntaxKind::PrefixExpression,
            ("projected_type", true) => SyntaxKind::ProjectedType,
            ("quote_expression", true) => SyntaxKind::QuoteExpression,
            ("refinement", true) => SyntaxKind::Refinement,
            ("repeat_pattern", true) => SyntaxKind::RepeatPattern,
            ("repeated_parameter_type", true) => SyntaxKind::RepeatedParameterType,
            ("return_expression", true) => SyntaxKind::ReturnExpression,
            ("self_type", true) => SyntaxKind::SelfType,
            ("simple_enum_case", true) => SyntaxKind::SimpleEnumCase,
            ("singleton_type", true) => SyntaxKind::SingletonType,
            ("splice_expression", true) => SyntaxKind::SpliceExpression,
            ("stable_identifier", true) => SyntaxKind::StableIdentifier,
            ("stable_type_identifier", true) => SyntaxKind::StableTypeIdentifier,
            ("string", true) => SyntaxKind::String,
            ("structural_type", true) => SyntaxKind::StructuralType,
            ("template_body", true) => SyntaxKind::TemplateBody,
            ("throw_expression", true) => SyntaxKind::ThrowExpression,
            ("tracked_modifier", true) => SyntaxKind::TrackedModifier,
            ("trait_definition", true) => SyntaxKind::TraitDefinition,
            ("transparent_modifier", true) => SyntaxKind::TransparentModifier,
            ("try_expression", true) => SyntaxKind::TryExpression,
            ("tuple_expression", true) => SyntaxKind::TupleExpression,
            ("tuple_pattern", true) => SyntaxKind::TuplePattern,
            ("tuple_type", true) => SyntaxKind::TupleType,
            ("type_arguments", true) => SyntaxKind::TypeArguments,
            ("type_case_clause", true) => SyntaxKind::TypeCaseClause,
            ("type_definition", true) => SyntaxKind::TypeDefinition,
            ("type_identifier", true) => SyntaxKind::TypeIdentifier,
            ("type_lambda", true) => SyntaxKind::TypeLambda,
            ("type_parameters", true) => SyntaxKind::TypeParameters,
            ("typed_pattern", true) => SyntaxKind::TypedPattern,
            ("unit", true) => SyntaxKind::Unit,
            ("update_modifier", true) => SyntaxKind::UpdateModifier,
            ("upper_bound", true) => SyntaxKind::UpperBound,
            ("uses_clause", true) => SyntaxKind::UsesClause,
            ("using_directive", true) => SyntaxKind::UsingDirective,
            ("using_directive_key", true) => SyntaxKind::UsingDirectiveKey,
            ("using_directive_value", true) => SyntaxKind::UsingDirectiveValue,
            ("val_declaration", true) => SyntaxKind::ValDeclaration,
            ("val_definition", true) => SyntaxKind::ValDefinition,
            ("var_declaration", true) => SyntaxKind::VarDeclaration,
            ("var_definition", true) => SyntaxKind::VarDefinition,
            ("vararg", true) => SyntaxKind::Vararg,
            ("view_bound", true) => SyntaxKind::ViewBound,
            ("while_expression", true) => SyntaxKind::WhileExpression,
            ("wildcard", true) => SyntaxKind::Wildcard,
            ("with_template_body", true) => SyntaxKind::WithTemplateBody,
            ("xml_attribute", true) => SyntaxKind::XmlAttribute,
            ("xml_cdata", true) => SyntaxKind::XmlCdata,
            ("xml_comment", true) => SyntaxKind::XmlComment,
            ("xml_element", true) => SyntaxKind::XmlElement,
            ("xml_expression", true) => SyntaxKind::XmlExpression,
            ("xml_name", true) => SyntaxKind::XmlName,
            ("xml_pattern", true) => SyntaxKind::XmlPattern,
            ("xml_processing_instruction", true) => SyntaxKind::XmlProcessingInstruction,
            ("xml_string", true) => SyntaxKind::XmlString,
            ("xml_text", true) => SyntaxKind::XmlText,
            ("!", false) => SyntaxKind::Bang,
            ("\"", false) => SyntaxKind::Quote,
            ("\"\"\"", false) => SyntaxKind::QuoteQuoteQuote,
            ("#", false) => SyntaxKind::Pound,
            ("$", false) => SyntaxKind::Dollar,
            ("$[", false) => SyntaxKind::DollarLBrack,
            ("${", false) => SyntaxKind::DollarLCurly,
            ("'", false) => SyntaxKind::Apos,
            ("(", false) => SyntaxKind::LParen,
            (")", false) => SyntaxKind::RParen,
            ("*", false) => SyntaxKind::Star,
            ("+", false) => SyntaxKind::Plus,
            (",", false) => SyntaxKind::Comma,
            ("-", false) => SyntaxKind::Minus,
            ("->", false) => SyntaxKind::MinusGt,
            (".", false) => SyntaxKind::Dot,
            ("//", false) => SyntaxKind::SlashSlash,
            ("/>", false) => SyntaxKind::SlashGt,
            (":", false) => SyntaxKind::Colon,
            (";", false) => SyntaxKind::Semi,
            ("<", false) => SyntaxKind::Lt,
            ("<%", false) => SyntaxKind::LtPercent,
            ("<-", false) => SyntaxKind::LtMinus,
            ("</", false) => SyntaxKind::LtSlash,
            ("<:", false) => SyntaxKind::LtColon,
            ("=", false) => SyntaxKind::Eq,
            ("=>", false) => SyntaxKind::EqGt,
            ("=>>", false) => SyntaxKind::EqGtGt,
            (">", false) => SyntaxKind::Gt,
            (">:", false) => SyntaxKind::GtColon,
            ("?->", false) => SyntaxKind::QuestionMinusGt,
            ("?=>", false) => SyntaxKind::QuestionEqGt,
            ("@", false) => SyntaxKind::At,
            ("[", false) => SyntaxKind::LBrack,
            ("]", false) => SyntaxKind::RBrack,
            ("^", false) => SyntaxKind::Caret,
            ("_", false) => SyntaxKind::Underscore,
            ("_end_ident", false) => SyntaxKind::EndIdentKw,
            ("abstract", false) => SyntaxKind::AbstractKw,
            ("as", false) => SyntaxKind::AsKw,
            ("cap", false) => SyntaxKind::CapKw,
            ("case", false) => SyntaxKind::CaseKw,
            ("catch", false) => SyntaxKind::CatchKw,
            ("class", false) => SyntaxKind::ClassKw,
            ("def", false) => SyntaxKind::DefKw,
            ("derives", false) => SyntaxKind::DerivesKw,
            ("do", false) => SyntaxKind::DoKw,
            ("else", false) => SyntaxKind::ElseKw,
            ("enum", false) => SyntaxKind::EnumKw,
            ("except", false) => SyntaxKind::ExceptKw,
            ("export", false) => SyntaxKind::ExportKw,
            ("extends", false) => SyntaxKind::ExtendsKw,
            ("extension", false) => SyntaxKind::ExtensionKw,
            ("false", false) => SyntaxKind::FalseKw,
            ("final", false) => SyntaxKind::FinalKw,
            ("finally", false) => SyntaxKind::FinallyKw,
            ("for", false) => SyntaxKind::ForKw,
            ("forSome", false) => SyntaxKind::ForSomeKw,
            ("given", false) => SyntaxKind::GivenKw,
            ("if", false) => SyntaxKind::IfKw,
            ("implicit", false) => SyntaxKind::ImplicitKw,
            ("import", false) => SyntaxKind::ImportKw,
            ("lazy", false) => SyntaxKind::LazyKw,
            ("macro", false) => SyntaxKind::MacroKw,
            ("match", false) => SyntaxKind::MatchKw,
            ("new", false) => SyntaxKind::NewKw,
            ("object", false) => SyntaxKind::ObjectKw,
            ("only", false) => SyntaxKind::OnlyKw,
            ("override", false) => SyntaxKind::OverrideKw,
            ("package", false) => SyntaxKind::PackageKw,
            ("private", false) => SyntaxKind::PrivateKw,
            ("protected", false) => SyntaxKind::ProtectedKw,
            ("rd", false) => SyntaxKind::RdKw,
            ("return", false) => SyntaxKind::ReturnKw,
            ("sealed", false) => SyntaxKind::SealedKw,
            ("super", false) => SyntaxKind::SuperKw,
            ("then", false) => SyntaxKind::ThenKw,
            ("this", false) => SyntaxKind::ThisKw,
            ("throw", false) => SyntaxKind::ThrowKw,
            ("trait", false) => SyntaxKind::TraitKw,
            ("true", false) => SyntaxKind::TrueKw,
            ("try", false) => SyntaxKind::TryKw,
            ("type", false) => SyntaxKind::TypeKw,
            ("uses", false) => SyntaxKind::UsesKw,
            ("using", false) => SyntaxKind::UsingKw,
            ("val", false) => SyntaxKind::ValKw,
            ("var", false) => SyntaxKind::VarKw,
            ("while", false) => SyntaxKind::WhileKw,
            ("with", false) => SyntaxKind::WithKw,
            ("yield", false) => SyntaxKind::YieldKw,
            ("{", false) => SyntaxKind::LCurly,
            ("|", false) => SyntaxKind::Pipe,
            ("}", false) => SyntaxKind::RCurly,
            ("~", false) => SyntaxKind::Tilde,
            ("←", false) => SyntaxKind::UnicodeLeftArrow,
            _ => return None,
        })
    }
}

/// A keyword or punctuation kind: `T![class]`, `T!["{"]`, `T!["=>"]`.
#[macro_export]
macro_rules! T {
    ["!"] => { $crate::lossless_tree::SyntaxKind::Bang };
    ["\""] => { $crate::lossless_tree::SyntaxKind::Quote };
    ["\"\"\""] => { $crate::lossless_tree::SyntaxKind::QuoteQuoteQuote };
    ["#"] => { $crate::lossless_tree::SyntaxKind::Pound };
    ["$"] => { $crate::lossless_tree::SyntaxKind::Dollar };
    ["$["] => { $crate::lossless_tree::SyntaxKind::DollarLBrack };
    ["${"] => { $crate::lossless_tree::SyntaxKind::DollarLCurly };
    ["'"] => { $crate::lossless_tree::SyntaxKind::Apos };
    ["("] => { $crate::lossless_tree::SyntaxKind::LParen };
    [")"] => { $crate::lossless_tree::SyntaxKind::RParen };
    ["*"] => { $crate::lossless_tree::SyntaxKind::Star };
    ["+"] => { $crate::lossless_tree::SyntaxKind::Plus };
    [","] => { $crate::lossless_tree::SyntaxKind::Comma };
    ["-"] => { $crate::lossless_tree::SyntaxKind::Minus };
    ["->"] => { $crate::lossless_tree::SyntaxKind::MinusGt };
    ["."] => { $crate::lossless_tree::SyntaxKind::Dot };
    ["//"] => { $crate::lossless_tree::SyntaxKind::SlashSlash };
    ["/>"] => { $crate::lossless_tree::SyntaxKind::SlashGt };
    [":"] => { $crate::lossless_tree::SyntaxKind::Colon };
    [";"] => { $crate::lossless_tree::SyntaxKind::Semi };
    ["<"] => { $crate::lossless_tree::SyntaxKind::Lt };
    ["<%"] => { $crate::lossless_tree::SyntaxKind::LtPercent };
    ["<-"] => { $crate::lossless_tree::SyntaxKind::LtMinus };
    ["</"] => { $crate::lossless_tree::SyntaxKind::LtSlash };
    ["<:"] => { $crate::lossless_tree::SyntaxKind::LtColon };
    ["="] => { $crate::lossless_tree::SyntaxKind::Eq };
    ["=>"] => { $crate::lossless_tree::SyntaxKind::EqGt };
    ["=>>"] => { $crate::lossless_tree::SyntaxKind::EqGtGt };
    [">"] => { $crate::lossless_tree::SyntaxKind::Gt };
    [">:"] => { $crate::lossless_tree::SyntaxKind::GtColon };
    ["?->"] => { $crate::lossless_tree::SyntaxKind::QuestionMinusGt };
    ["?=>"] => { $crate::lossless_tree::SyntaxKind::QuestionEqGt };
    ["@"] => { $crate::lossless_tree::SyntaxKind::At };
    ["["] => { $crate::lossless_tree::SyntaxKind::LBrack };
    ["]"] => { $crate::lossless_tree::SyntaxKind::RBrack };
    ["^"] => { $crate::lossless_tree::SyntaxKind::Caret };
    ["_"] => { $crate::lossless_tree::SyntaxKind::Underscore };
    [_end_ident] => { $crate::lossless_tree::SyntaxKind::EndIdentKw };
    ["_end_ident"] => { $crate::lossless_tree::SyntaxKind::EndIdentKw };
    [abstract] => { $crate::lossless_tree::SyntaxKind::AbstractKw };
    ["abstract"] => { $crate::lossless_tree::SyntaxKind::AbstractKw };
    [as] => { $crate::lossless_tree::SyntaxKind::AsKw };
    ["as"] => { $crate::lossless_tree::SyntaxKind::AsKw };
    [cap] => { $crate::lossless_tree::SyntaxKind::CapKw };
    ["cap"] => { $crate::lossless_tree::SyntaxKind::CapKw };
    [case] => { $crate::lossless_tree::SyntaxKind::CaseKw };
    ["case"] => { $crate::lossless_tree::SyntaxKind::CaseKw };
    [catch] => { $crate::lossless_tree::SyntaxKind::CatchKw };
    ["catch"] => { $crate::lossless_tree::SyntaxKind::CatchKw };
    [class] => { $crate::lossless_tree::SyntaxKind::ClassKw };
    ["class"] => { $crate::lossless_tree::SyntaxKind::ClassKw };
    [def] => { $crate::lossless_tree::SyntaxKind::DefKw };
    ["def"] => { $crate::lossless_tree::SyntaxKind::DefKw };
    [derives] => { $crate::lossless_tree::SyntaxKind::DerivesKw };
    ["derives"] => { $crate::lossless_tree::SyntaxKind::DerivesKw };
    [do] => { $crate::lossless_tree::SyntaxKind::DoKw };
    ["do"] => { $crate::lossless_tree::SyntaxKind::DoKw };
    [else] => { $crate::lossless_tree::SyntaxKind::ElseKw };
    ["else"] => { $crate::lossless_tree::SyntaxKind::ElseKw };
    [enum] => { $crate::lossless_tree::SyntaxKind::EnumKw };
    ["enum"] => { $crate::lossless_tree::SyntaxKind::EnumKw };
    [except] => { $crate::lossless_tree::SyntaxKind::ExceptKw };
    ["except"] => { $crate::lossless_tree::SyntaxKind::ExceptKw };
    [export] => { $crate::lossless_tree::SyntaxKind::ExportKw };
    ["export"] => { $crate::lossless_tree::SyntaxKind::ExportKw };
    [extends] => { $crate::lossless_tree::SyntaxKind::ExtendsKw };
    ["extends"] => { $crate::lossless_tree::SyntaxKind::ExtendsKw };
    [extension] => { $crate::lossless_tree::SyntaxKind::ExtensionKw };
    ["extension"] => { $crate::lossless_tree::SyntaxKind::ExtensionKw };
    [false] => { $crate::lossless_tree::SyntaxKind::FalseKw };
    ["false"] => { $crate::lossless_tree::SyntaxKind::FalseKw };
    [final] => { $crate::lossless_tree::SyntaxKind::FinalKw };
    ["final"] => { $crate::lossless_tree::SyntaxKind::FinalKw };
    [finally] => { $crate::lossless_tree::SyntaxKind::FinallyKw };
    ["finally"] => { $crate::lossless_tree::SyntaxKind::FinallyKw };
    [for] => { $crate::lossless_tree::SyntaxKind::ForKw };
    ["for"] => { $crate::lossless_tree::SyntaxKind::ForKw };
    [forSome] => { $crate::lossless_tree::SyntaxKind::ForSomeKw };
    ["forSome"] => { $crate::lossless_tree::SyntaxKind::ForSomeKw };
    [given] => { $crate::lossless_tree::SyntaxKind::GivenKw };
    ["given"] => { $crate::lossless_tree::SyntaxKind::GivenKw };
    [if] => { $crate::lossless_tree::SyntaxKind::IfKw };
    ["if"] => { $crate::lossless_tree::SyntaxKind::IfKw };
    [implicit] => { $crate::lossless_tree::SyntaxKind::ImplicitKw };
    ["implicit"] => { $crate::lossless_tree::SyntaxKind::ImplicitKw };
    [import] => { $crate::lossless_tree::SyntaxKind::ImportKw };
    ["import"] => { $crate::lossless_tree::SyntaxKind::ImportKw };
    [lazy] => { $crate::lossless_tree::SyntaxKind::LazyKw };
    ["lazy"] => { $crate::lossless_tree::SyntaxKind::LazyKw };
    [macro] => { $crate::lossless_tree::SyntaxKind::MacroKw };
    ["macro"] => { $crate::lossless_tree::SyntaxKind::MacroKw };
    [match] => { $crate::lossless_tree::SyntaxKind::MatchKw };
    ["match"] => { $crate::lossless_tree::SyntaxKind::MatchKw };
    [new] => { $crate::lossless_tree::SyntaxKind::NewKw };
    ["new"] => { $crate::lossless_tree::SyntaxKind::NewKw };
    [object] => { $crate::lossless_tree::SyntaxKind::ObjectKw };
    ["object"] => { $crate::lossless_tree::SyntaxKind::ObjectKw };
    [only] => { $crate::lossless_tree::SyntaxKind::OnlyKw };
    ["only"] => { $crate::lossless_tree::SyntaxKind::OnlyKw };
    [override] => { $crate::lossless_tree::SyntaxKind::OverrideKw };
    ["override"] => { $crate::lossless_tree::SyntaxKind::OverrideKw };
    [package] => { $crate::lossless_tree::SyntaxKind::PackageKw };
    ["package"] => { $crate::lossless_tree::SyntaxKind::PackageKw };
    [private] => { $crate::lossless_tree::SyntaxKind::PrivateKw };
    ["private"] => { $crate::lossless_tree::SyntaxKind::PrivateKw };
    [protected] => { $crate::lossless_tree::SyntaxKind::ProtectedKw };
    ["protected"] => { $crate::lossless_tree::SyntaxKind::ProtectedKw };
    [rd] => { $crate::lossless_tree::SyntaxKind::RdKw };
    ["rd"] => { $crate::lossless_tree::SyntaxKind::RdKw };
    [return] => { $crate::lossless_tree::SyntaxKind::ReturnKw };
    ["return"] => { $crate::lossless_tree::SyntaxKind::ReturnKw };
    [sealed] => { $crate::lossless_tree::SyntaxKind::SealedKw };
    ["sealed"] => { $crate::lossless_tree::SyntaxKind::SealedKw };
    [super] => { $crate::lossless_tree::SyntaxKind::SuperKw };
    ["super"] => { $crate::lossless_tree::SyntaxKind::SuperKw };
    [then] => { $crate::lossless_tree::SyntaxKind::ThenKw };
    ["then"] => { $crate::lossless_tree::SyntaxKind::ThenKw };
    [this] => { $crate::lossless_tree::SyntaxKind::ThisKw };
    ["this"] => { $crate::lossless_tree::SyntaxKind::ThisKw };
    [throw] => { $crate::lossless_tree::SyntaxKind::ThrowKw };
    ["throw"] => { $crate::lossless_tree::SyntaxKind::ThrowKw };
    [trait] => { $crate::lossless_tree::SyntaxKind::TraitKw };
    ["trait"] => { $crate::lossless_tree::SyntaxKind::TraitKw };
    [true] => { $crate::lossless_tree::SyntaxKind::TrueKw };
    ["true"] => { $crate::lossless_tree::SyntaxKind::TrueKw };
    [try] => { $crate::lossless_tree::SyntaxKind::TryKw };
    ["try"] => { $crate::lossless_tree::SyntaxKind::TryKw };
    [type] => { $crate::lossless_tree::SyntaxKind::TypeKw };
    ["type"] => { $crate::lossless_tree::SyntaxKind::TypeKw };
    [uses] => { $crate::lossless_tree::SyntaxKind::UsesKw };
    ["uses"] => { $crate::lossless_tree::SyntaxKind::UsesKw };
    [using] => { $crate::lossless_tree::SyntaxKind::UsingKw };
    ["using"] => { $crate::lossless_tree::SyntaxKind::UsingKw };
    [val] => { $crate::lossless_tree::SyntaxKind::ValKw };
    ["val"] => { $crate::lossless_tree::SyntaxKind::ValKw };
    [var] => { $crate::lossless_tree::SyntaxKind::VarKw };
    ["var"] => { $crate::lossless_tree::SyntaxKind::VarKw };
    [while] => { $crate::lossless_tree::SyntaxKind::WhileKw };
    ["while"] => { $crate::lossless_tree::SyntaxKind::WhileKw };
    [with] => { $crate::lossless_tree::SyntaxKind::WithKw };
    ["with"] => { $crate::lossless_tree::SyntaxKind::WithKw };
    [yield] => { $crate::lossless_tree::SyntaxKind::YieldKw };
    ["yield"] => { $crate::lossless_tree::SyntaxKind::YieldKw };
    ["{"] => { $crate::lossless_tree::SyntaxKind::LCurly };
    ["|"] => { $crate::lossless_tree::SyntaxKind::Pipe };
    ["}"] => { $crate::lossless_tree::SyntaxKind::RCurly };
    ["~"] => { $crate::lossless_tree::SyntaxKind::Tilde };
    ["←"] => { $crate::lossless_tree::SyntaxKind::UnicodeLeftArrow };
}
