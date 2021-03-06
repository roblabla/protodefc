use super::CompilePass;
use super::Result;

use ::{Type, TypeContainer, TypeVariant, TypeData, WeakTypeContainer, CompilerError};
use ::errors::*;
use ::FieldReference;

use std::rc::{Rc, Weak};
use std::cell::RefCell;

pub struct ResolveReferencePass;

impl CompilePass for ResolveReferencePass {
    fn run(typ: &mut TypeContainer) -> Result<()> {
        let mut parents: Vec<Weak<RefCell<Type>>> = Vec::new();
        do_run(typ, &mut parents)
    }
}

fn do_run(typ: &TypeContainer, parents: &mut Vec<Weak<RefCell<Type>>>)
          -> Result<()> {

    let parents_o = parents.clone();
    let resolver: &Fn(&TypeVariant, &TypeData, &FieldReference)
                      -> Result<WeakTypeContainer> =
        &move |variant, data, reference| {
            let chain = || CompilerError::ReferenceError {
                reference: reference.clone(),
            };

            if reference.up == 0 {
                variant.resolve_child_name(data, &reference.name)
                    .chain_err(chain)
            } else {
                if reference.up > parents_o.len() {
                    bail!(chain());
                }
                let root = &parents_o[parents_o.len() - 1 - (reference.up - 1)];
                let root_upgrade = root.upgrade().unwrap();
                let root_inner = root_upgrade.borrow_mut();

                let Type { ref data, ref variant } = *root_inner;
                variant.to_variant().resolve_child_name(data, &reference.name)
                    .chain_err(chain)
            }
    };

    let chain;
    let mut children;
    {
        let mut inner = typ.borrow_mut();
        children = inner.data.children.clone();

        chain = CompilerError::InsideVariant {
            variant: inner.variant.get_type(&inner.data),
        };

        let Type { ref mut data, ref mut variant } = *inner;
        variant.to_variant_mut()
            .do_resolve_references(data, resolver)
            .chain_err(|| chain.clone())?;
    }

    parents.push(Rc::downgrade(typ));
    for mut child in &mut children {
        do_run(&mut child, parents)
            .chain_err(|| chain.clone())?;
    }
    parents.pop();

    Ok(())
}
