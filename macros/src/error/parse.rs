// Dependencies

use proc_macro2::Span;
use syn::FieldsNamed;
use syn::FieldsUnnamed;
use syn::Ident;
use syn::LitStr;
use syn::Result;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;

// Keywords

mod keyword {
    use syn::custom_keyword;
    custom_keyword!(description);
    custom_keyword!(fields);
    custom_keyword!(causes);
}

// Data structures

pub struct Error {
    pub ident: Ident,
    pub description: LitStr,
    pub data: ErrorData
}

pub enum ErrorData {
    None,
    Fields(FieldsNamed),
    Causes(FieldsUnnamed)
}

enum ErrorProperty {
    Type(ErrorType),
    Description(ErrorDescription),
    Fields(ErrorFields),
    Causes(ErrorCauses)
}

struct ErrorType {
    type_token: Token![type],
    ident: Ident
}

struct ErrorDescription {
    description_token: keyword::description,
    description: LitStr
}

struct ErrorFields {
    fields_token: keyword::fields,
    fields: FieldsNamed
}

struct ErrorCauses {
    causes_token: keyword::causes,
    causes: FieldsUnnamed
}

// Implementations

impl Parse for Error {
    fn parse(input: ParseStream) -> Result<Self> {
        let properties: Punctuated<ErrorProperty, Token![,]> = input.parse_terminated(ErrorProperty::parse)?;

        let mut ident = Option::<Ident>::None;
        let mut description = Option::<LitStr>::None;
        let mut fields = Option::<FieldsNamed>::None;
        let mut causes = Option::<FieldsUnnamed>::None;

        for property in properties {
            match property {
                ErrorProperty::Type(property) => {
                    if ident.is_none() {
                        ident = Some(property.ident);
                    } else {
                        return Err(syn::Error::new(property.type_token.span, "Property `type` can only appear once in an `error!`."));
                    }
                },
                ErrorProperty::Description(property) => {
                    if description.is_none() {
                        description = Some(property.description);
                    } else {
                        return Err(syn::Error::new(property.description_token.span, "Property `description` can only appear once in an `error!`."));
                    }
                },
                ErrorProperty::Fields(property) => {
                    if fields.is_none() && causes.is_none() {
                        fields = Some(property.fields);
                    } else if fields.is_some() {
                        return Err(syn::Error::new(property.fields_token.span, "Property `fields` can only appear once in an `error!`."));
                    } else {
                        return Err(syn::Error::new(property.fields_token.span, "Properties `fields` and `causes` cannot appear together in an `error!`."));
                    }
                },
                ErrorProperty::Causes(property) => {
                    if causes.is_none() && fields.is_none() {
                        causes = Some(property.causes);
                    } else if causes.is_some() {
                        return Err(syn::Error::new(property.causes_token.span, "Property `causes` can only appear once in an `error!`."));
                    } else {
                        return Err(syn::Error::new(property.causes_token.span, "Properties `fields` and `causes` cannot appear together in an `error!`."));
                    }
                }
            }
        }

        if ident.is_none() {
            Err(syn::Error::new(Span::call_site(), "Property `type` is required in an `error!`."))
        } else if description.is_none() {
            Err(syn::Error::new(Span::call_site(), "Property `description` is required in an `error!`."))
        } else {
            Ok(Error{
                ident: ident.unwrap(),
                description: description.unwrap(),
                data: if fields.is_some() { ErrorData::Fields(fields.unwrap()) } else if causes.is_some() { ErrorData::Causes(causes.unwrap()) } else { ErrorData::None }
            })
        }
    }
}

impl Parse for ErrorProperty {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![type]) {
            input.parse().map(ErrorProperty::Type)
        } else if lookahead.peek(keyword::description) {
            input.parse().map(ErrorProperty::Description)
        } else if lookahead.peek(keyword::fields) {
            input.parse().map(ErrorProperty::Fields)
        } else if lookahead.peek(keyword::causes) {
            input.parse().map(ErrorProperty::Causes)
        } else {
            Err(lookahead.error())
        }
    }
}

impl Parse for ErrorType {
    fn parse(input: ParseStream) -> Result<Self> {
        let type_token = input.parse()?;
        let _: Token![:] = input.parse()?;
        let ident = input.parse()?;

        Ok(ErrorType{type_token, ident})
    }
}

impl Parse for ErrorDescription {
    fn parse(input: ParseStream) -> Result<Self> {
        let description_token = input.parse()?;
        let _: Token![:] = input.parse()?;
        let description = input.parse()?;

        Ok(ErrorDescription{description_token, description})
    }
}

impl Parse for ErrorFields {
    fn parse(input: ParseStream) -> Result<Self> {
        let fields_token = input.parse()?;
        let _: Token![:] = input.parse()?;
        let fields = input.parse()?;

        Ok(ErrorFields{fields_token, fields})
    }
}

impl Parse for ErrorCauses {
    fn parse(input: ParseStream) -> Result<Self> {
        let causes_token = input.parse()?;
        let _: Token![:] = input.parse()?;
        let causes = input.parse()?;

        Ok(ErrorCauses{causes_token, causes})
    }
}
