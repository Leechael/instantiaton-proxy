#![cfg_attr(not(feature = "std"), no_std, no_main)]
#[macro_use]
extern crate alloc;

#[ink::contract]
mod instantiate_proxy {
    use alloc::vec::Vec;
    use ink::env::{
        call::{build_create, ExecutionInput, Selector},
        ContractEnv, DefaultEnvironment,
    };
    use scale::{Decode, Encode};

    #[ink(storage)]
    pub struct InstantiateProxy {}

    #[derive(Encode, Decode, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        #[codec(index = 42)]
        Failed,
    }

    impl InstantiateProxy {
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
        #[ink(message)]
        pub fn instantiate(
            &mut self,
            code_hash: Hash,
            selector: [u8; 4],
            encoded_args: Vec<u8>,
            salt: Vec<u8>,
        ) -> Result<AccountId, Vec<u8>> {
            use types::{ConstructorError, ContractRef, Data};

            build_create::<ContractRef>()
                .code_hash(code_hash)
                .endowment(self.env().transferred_value())
                .exec_input(
                    ExecutionInput::new(Selector::new(selector)).push_arg(Data(encoded_args)),
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
        use ink::env::call::FromAccountId;

        pub type ConstructorError = Data;
        pub struct ContractRef(pub AccountId);
        impl ContractEnv for ContractRef {
            type Env = DefaultEnvironment;
        }
        impl FromAccountId<DefaultEnvironment> for ContractRef {
            fn from_account_id(account_id: AccountId) -> Self {
                Self(account_id)
            }
        }
        pub struct Data(pub Vec<u8>);
        impl Encode for Data {
            fn encode_to<T: scale::Output + ?Sized>(&self, dest: &mut T) {
                dest.write(&self.0)
            }
        }
        impl Decode for Data {
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

            let wasm = include_bytes!("./target/ink/instantiate_proxy.wasm");
            let code_hash = code_hash(wasm);
            let mut session =
                drink::session::Session::<drink_pink_runtime::PinkRuntime>::new().unwrap();
            let mut proxy = InstantiateProxyRef::new(true)
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