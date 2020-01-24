use proc_macro2::Span;

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{FieldsNamed, FieldsUnnamed, Ident, LitStr, Result, Token};

mod keyword {
    use syn::custom_keyword;
    custom_keyword!(description);
    custom_keyword!(fields);
    custom_keyword!(causes);
}

pub struct Error {
    pub idents: Idents,
    pub description: LitStr,
    pub data: ErrorData,
}

pub struct Idents {
    pub error: Ident,
    pub cause: Ident,
}

pub enum ErrorData {
    None,
    Fields(FieldsNamed),
    Causes(FieldsUnnamed),
}

enum ErrorProperty {
    Type(ErrorType),
    Description(ErrorDescription),
    Fields(ErrorFields),
    Causes(ErrorCauses),
}

struct ErrorType {
    type_token: Token![type],
    ident: Ident,
}

struct ErrorDescription {
    description_token: keyword::description,
    description: LitStr,
}

struct ErrorFields {
    fields_token: keyword::fields,
    fields: FieldsNamed,
}

struct ErrorCauses {
    causes_token: keyword::causes,
    causes: FieldsUnnamed,
}

impl Parse for Error {
    fn parse(input: ParseStream) -> Result<Self> {
        let properties: Punctuated<ErrorProperty, Token![,]> =
            input.parse_terminated(ErrorProperty::parse)?;

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
                }
                ErrorProperty::Description(property) => {
                    if description.is_none() {
                        description = Some(property.description);
                    } else {
                        return Err(syn::Error::new(property.description_token.span, "Property `description` can only appear once in an `error!`."));
                    }
                }
                ErrorProperty::Fields(property) => {
                    if fields.is_none() && causes.is_none() {
                        fields = Some(property.fields);
                    } else if fields.is_some() {
                        return Err(syn::Error::new(property.fields_token.span, "Property `fields` can only appear once in an `error!`."));
                    } else {
                        return Err(syn::Error::new(property.fields_token.span, "Properties `fields` and `causes` cannot appear together in an `error!`."));
                    }
                }
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

        match (ident, description) {
            (None, _) => Err(syn::Error::new(
                Span::call_site(),
                "Property `type` is required in an `error!`.",
            )),
            (_, None) => Err(syn::Error::new(
                Span::call_site(),
                "Property `description` is required in an `error!`.",
            )),

            (Some(ident), Some(description)) => {
                let error = ident;
                let cause =
                    Ident::new(&format!("{}Cause", error), error.span());

                Ok(Error {
                    idents: Idents { error, cause },
                    description,
                    data: if let Some(fields) = fields {
                        ErrorData::Fields(fields)
                    } else if let Some(causes) = causes {
                        ErrorData::Causes(causes)
                    } else {
                        ErrorData::None
                    },
                })
            }
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

        Ok(ErrorType { type_token, ident })
    }
}

impl Parse for ErrorDescription {
    fn parse(input: ParseStream) -> Result<Self> {
        let description_token = input.parse()?;
        let _: Token![:] = input.parse()?;
        let description = input.parse()?;

        Ok(ErrorDescription {
            description_token,
            description,
        })
    }
}

impl Parse for ErrorFields {
    fn parse(input: ParseStream) -> Result<Self> {
        let fields_token = input.parse()?;
        let _: Token![:] = input.parse()?;
        let fields = input.parse()?;

        Ok(ErrorFields {
            fields_token,
            fields,
        })
    }
}

impl Parse for ErrorCauses {
    fn parse(input: ParseStream) -> Result<Self> {
        let causes_token = input.parse()?;
        let _: Token![:] = input.parse()?;
        let causes = input.parse()?;

        Ok(ErrorCauses {
            causes_token,
            causes,
        })
    }
}
