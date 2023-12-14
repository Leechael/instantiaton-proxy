#![cfg_attr(not(feature = "std"), no_std, no_main)]
#[macro_use]
extern crate alloc;

#[ink::contract]
mod instantiation_proxy {
    use alloc::vec::Vec;
    use ink::env::call::{build_create, ExecutionInput, Selector};
    use scale::{Decode, Encode};

    type EncodedConstructorError = Vec<u8>;

    #[ink(storage)]
    pub struct InstantiationProxy {}

    #[derive(Encode, Decode, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        #[codec(index = 42)]
        Failed,
    }

    impl InstantiationProxy {
        #[ink(constructor)]
        pub fn new(success: bool) -> Self {
            if success {
                Self {}
            } else {
                panic!("panic in constructor")
            }
        }

        #[ink(constructor)]
        pub fn new_failable(success: bool) -> Result<Self, Error> {
            if success {
                Ok(Self {})
            } else {
                Err(Error::Failed)
            }
        }

        /// Proxy a contract instantiation
        ///
        /// Returns Ok(address) on success, Err(error) on failure where error is the original
        /// constructor error encoded in SCALE.
        #[ink(message)]
        pub fn instantiate(
            &mut self,
            code_hash: Hash,
            constructor_selector: [u8; 4],
            encoded_args: Vec<u8>,
            salt: Vec<u8>,
        ) -> Result<AccountId, EncodedConstructorError> {
            use types::{ConstructorError, ContractRef, Encoded};

            build_create::<ContractRef>()
                .code_hash(code_hash)
                .endowment(self.env().transferred_value())
                .exec_input(
                    ExecutionInput::new(Selector::new(constructor_selector))
                        .push_arg(Encoded(encoded_args)),
                )
                .salt_bytes(&salt)
                .returns::<Result<ContractRef, ConstructorError>>()
                .instantiate()
                .map(|contract_ref| contract_ref.0)
                .map_err(|err| err.0)
        }
    }

    mod types {
        use super::*;
        use ink::env::{call::FromAccountId, ContractEnv, DefaultEnvironment};

        pub type ConstructorError = Encoded;
        pub struct ContractRef(pub AccountId);
        impl ContractEnv for ContractRef {
            type Env = DefaultEnvironment;
        }
        impl FromAccountId<DefaultEnvironment> for ContractRef {
            fn from_account_id(account_id: AccountId) -> Self {
                Self(account_id)
            }
        }
        pub struct Encoded(pub Vec<u8>);
        impl Encode for Encoded {
            fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
                dest.write(&self.0)
            }
        }
        impl Decode for Encoded {
            fn decode<I: scale::Input>(input: &mut I) -> Result<Self, scale::Error> {
                let mut buffer = vec![0u8; input.remaining_len()?.ok_or("unknown input length")?];
                input.read(&mut buffer[..])?;
                Ok(Self(buffer))
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[ink::test]
        fn it_works() {
            use drink_pink_runtime::{code_hash, Callable, DeployBundle};
            use ink::codegen::TraitCallBuilder;

            let wasm = include_bytes!("./target/ink/instantiation_proxy.wasm");
            let code_hash = code_hash(wasm);
            let mut session =
                drink::session::Session::<drink_pink_runtime::PinkRuntime>::new().unwrap();
            let mut proxy = InstantiationProxyRef::new(true)
                .deploy_wasm(wasm, &mut session)
                .unwrap();

            let result = proxy
                .call_mut()
                .instantiate(
                    code_hash.into(),
                    ink::selector_bytes!("new"),
                    (true,).encode(),
                    vec![0x00],
                )
                .submit_tx(&mut session);
            assert!(matches!(result, Ok(Ok(_))));

            let result = proxy
                .call_mut()
                .instantiate(
                    code_hash.into(),
                    ink::selector_bytes!("new"),
                    (false,).encode(),
                    vec![0x01],
                )
                .submit_tx(&mut session);
            assert!(matches!(result, Err(_)));

            let result = proxy
                .call_mut()
                .instantiate(
                    code_hash.into(),
                    ink::selector_bytes!("new_failable"),
                    (true,).encode(),
                    vec![0x02],
                )
                .submit_tx(&mut session);
            assert!(matches!(result, Ok(Ok(_))));

            let result = proxy
                .call_mut()
                .instantiate(
                    code_hash.into(),
                    ink::selector_bytes!("new_failable"),
                    (false,).encode(),
                    vec![0x03],
                )
                .submit_tx(&mut session);
            assert!(matches!(result, Ok(Err(_))));
            let err = result.ok().unwrap().err().unwrap();
            let decoded_err = super::Error::decode(&mut &err[..]).unwrap();
            assert_eq!(decoded_err, super::Error::Failed);
        }
    }
}
