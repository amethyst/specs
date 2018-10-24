use proc_macro2::TokenStream;
use syn::punctuated::Pair;
use syn::{
    ArgCaptured, AttributeArgs, FnArg, GenericArgument, ItemFn, Meta, NestedMeta, Path,
    PathArguments, ReturnType, Type, TypePath,
};

/// The main entrypoint, sets things up and delegates to the
/// type we're deriving on.
pub fn impl_specs_system(args: AttributeArgs, input: ItemFn) -> TokenStream {
    let system_name = if args.len() == 1 {
        match args.into_iter().next().unwrap() {
            NestedMeta::Meta(Meta::Word(name)) => name,
            arg @ _ => panic!(
                "Expected an identifier for a struct name such as `MyStruct`, found: `{:?}`.",
                arg
            ),
        }
    } else {
        panic!(
            "Expected exactly one argument to `specs_system` attribute, found: `{:?}`",
            args
        )
    };

    let decl = input.decl;
    if let ReturnType::Type(_, ty) = decl.output {
        panic!(
            "System function must have default `()` return type, found: `{:?}`",
            ty
        );
    }

    let system_lifetime: GenericArgument = parse_quote!('a);;

    let (system_data_names, system_data_types) = decl
        .inputs
        .iter()
        // Filter the function parameter types
        .filter_map(|fn_arg| match fn_arg {
            FnArg::SelfRef(..) | FnArg::SelfValue(..) => {
                panic!("System function must not have a `self` parameter.")
            }
            FnArg::Captured(ArgCaptured { pat, ty, .. }) => Some((pat, ty)),
            FnArg::Ignored(_ty) => unimplemented!(),
            _ => None,
        })
        // Prepend lifetime parameter
        .fold(
            (
                Vec::with_capacity(decl.inputs.len()),
                Vec::with_capacity(decl.inputs.len()),
            ),
            |(mut names, mut types), (pat, ty)| {
                let mut ty = ty.clone();

                {
                    let segments = match ty {
                        Type::Path(TypePath {
                            path:
                                Path {
                                    ref mut segments, ..
                                },
                            ..
                        }) => segments,
                        Type::Tuple(_) => unimplemented!(), // TODO: recurse
                        _ => panic!("Unexpected type, expected Path and Tuple, found: {:?}", &ty),
                    };

                    let last_segment = segments
                        .last_mut()
                        .expect("Expected path to contain last segment.");

                    let segment = match last_segment {
                        Pair::End(segment) => segment,
                        _ => unreachable!(),
                    };

                    let amended_arguments = {
                        let arguments = &segment.arguments;
                        match arguments {
                            PathArguments::None => {
                                let lifetime_generic_arg = parse_quote!(< #system_lifetime >);
                                PathArguments::AngleBracketed(lifetime_generic_arg)
                            }
                            PathArguments::AngleBracketed(ref generic_args) => {
                                let mut generic_args = generic_args.clone();
                                generic_args.args.insert(0, system_lifetime.clone());
                                PathArguments::AngleBracketed(generic_args)
                            }
                            PathArguments::Parenthesized(_) => {
                                panic!("Unexpected type argument for SystemData.")
                            }
                        }
                    };

                    segment.arguments = amended_arguments;
                }

                names.push(pat);
                types.push(ty);

                (names, types)
            },
        );

    let block = input.block;

    quote! {
        struct #system_name;

        impl<'a> System<'a> for #system_name {
            type SystemData = (#(#system_data_types),*);

            fn run(&mut self, (#(#system_data_names),*): Self::SystemData)
            #block
        }
    }
}
