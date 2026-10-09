use proc_macro::TokenStream;
use quote::quote;
use syn::{
    FnArg, GenericArgument, ItemFn, PatType, PathArguments, Signature, Type, TypePath,
    TypeReference, TypeTuple, parse_macro_input,
};

#[proc_macro_attribute]
pub fn animation(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    let anim_type = match extract_animation_type(&input_fn.sig) {
        Ok(t) => t,
        Err(e) => return e.to_compile_error().into(),
    };

    let fn_name = &input_fn.sig.ident;

    let expanded = quote! {
        #input_fn

        impl ::bevy_ecs_animations::Animation for #anim_type {
            fn system() -> (
                impl ::bevy::ecs::schedule::ScheduleLabel,
                ::bevy_ecs_animations::TickSystemConfigs,
            ) {
                (::bevy::app::Update, #fn_name.into_configs())
            }

            fn configuration(&self) -> impl ::std::convert::Into<::bevy_ecs_animations::AnimationConfiguration> {
                ::bevy_ecs_animations::forever()
            }
        }
    };

    expanded.into()
}

fn extract_animation_type(sig: &Signature) -> syn::Result<Type> {
    for arg in &sig.inputs {
        if let FnArg::Typed(PatType { ty, .. }) = arg {
            if let Some(found) = find_tick_in_type(ty) {
                return Ok(found);
            }
        }
    }
    Err(syn::Error::new_spanned(
        sig,
        "No parameter containing `Tick<A>` or `PersistentTick<A>` found. Expected something like \
         Single<(&mut Transform, &Tick<MyAnimation>)> or Query<&PersistentTick<MyAnimation>>",
    ))
}

fn find_tick_in_type(ty: &Type) -> Option<Type> {
    match ty {
        // We only care about &Tick<T> (immutable)
        Type::Reference(TypeReference {
            elem,
            mutability: None,
            ..
        }) => {
            return find_tick_in_type(elem);
        }

        Type::Path(TypePath { qself: None, path }) => {
            let last = path.segments.last()?;

            // Found a tick component
            if last.ident == "Tick" || last.ident == "PersistentTick" {
                if let PathArguments::AngleBracketed(args) = &last.arguments {
                    if let Some(GenericArgument::Type(t)) = args.args.first() {
                        return Some(t.clone());
                    }
                }
            }

            // Recurse into other generic parameters (Single<...>, Query<...>, etc.)
            for seg in &path.segments {
                if let PathArguments::AngleBracketed(ab) = &seg.arguments {
                    for arg in &ab.args {
                        if let GenericArgument::Type(inner) = arg {
                            if let Some(found) = find_tick_in_type(inner) {
                                return Some(found);
                            }
                        }
                    }
                }
            }
        }

        Type::Tuple(TypeTuple { elems, .. }) => {
            for elem in elems {
                if let Some(found) = find_tick_in_type(elem) {
                    return Some(found);
                }
            }
        }

        _ => {}
    }
    None
}
