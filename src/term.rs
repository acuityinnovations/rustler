use ::{NifDecoder, NifEnv, NifResult};
use ::wrapper::nif_interface::NIF_TERM;
use ::wrapper::env::term_to_binary;
use ::types::binary::NifBinary;
use std::fmt::{self, Debug};

/// NifTerm is used to represent all erlang terms. Terms are always lifetime limited by a NifEnv.
///
/// NifTerm is cloneable and copyable, but it can not exist outside of the lifetime of the NifEnv
/// that owns it.
#[derive(Clone, Copy)]
pub struct NifTerm<'a> {
    term: NIF_TERM,
    env: NifEnv<'a>,
}

impl<'a> Debug for NifTerm<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        ::wrapper::term::fmt(self.as_c_arg(), f)
    }
}

impl<'a> NifTerm<'a> {

    /// Create a `NifTerm` from a raw `NIF_TERM`.
    ///
    /// # Unsafe
    /// The caller must ensure that `env` is the environment that `inner` belongs to,
    /// unless `inner` is an atom term.
    pub unsafe fn new(env: NifEnv<'a>, inner: NIF_TERM) -> Self {
        NifTerm {
            term: inner,
            env: env,
        }
    }
    /// This extracts the raw term pointer. It is usually used in order to obtain a type that can
    /// be passed to calls into the erlang vm.
    pub fn as_c_arg(&self) -> NIF_TERM {
        self.term
    }

    pub fn get_env(&self) -> NifEnv<'a> {
        self.env
    }

    /// Returns a representation of self in the given NifEnv.
    ///
    /// If the term is already is in the provided env, it will be directly returned. Otherwise
    /// the term will be copied over.
    pub fn in_env<'b>(&self, env: NifEnv<'b>) -> NifTerm<'b> {
        if self.get_env() == env {
            // It's safe to create a new NifTerm<'b> without copying because we
            // just proved that the same environment is associated with both 'a
            // and 'b.  (They are either exactly the same lifetime, or the
            // lifetimes of two .run() calls on the same OwnedEnv.)
            unsafe { NifTerm::new(env, self.as_c_arg()) }
        } else {
            unsafe { NifTerm::new(env, ::wrapper::copy_term(env.as_c_arg(), self.as_c_arg())) }
        }
    }

    /// Decodes the NifTerm into type T.
    ///
    /// This should be used as the primary method of extracting the value from a NifTerm.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let term: NifTerm = ...;
    /// let number: i32 = try!(term.decode());
    /// ```
    pub fn decode<T>(self) -> NifResult<T> where T: NifDecoder<'a> {
        NifDecoder::decode(self)
    }

    pub fn to_binary(self) -> NifBinary<'a> {
        let raw_binary = unsafe { term_to_binary(self.env.as_c_arg(), self.as_c_arg()) }.unwrap();
        unsafe { NifBinary::from_raw(self.env, raw_binary) }
    }
}
