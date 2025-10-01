use anyhow::anyhow;
use soroban_env_common::Val;
use soroban_env_host::{
    budget::{AsBudget, Budget},
    xdr::ContractCostParams,
    CompilationContext, ErrorHandler, HostError, ModuleCache,
};

pub fn new_module_cache() -> anyhow::Result<(ModuleCache, CoreCompilationContext)> {
    let ctx =
        CoreCompilationContext::new().map_err(|e| anyhow!("error creating module cache: {}", e))?;
    let cache = ModuleCache::new(&ctx)?;

    Ok((cache, ctx))
}

#[derive(Clone)]
pub struct CoreCompilationContext {
    unlimited_budget: Budget,
}

impl CompilationContext for CoreCompilationContext {}

#[allow(dead_code)]
impl CoreCompilationContext {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let unlimited_budget = Budget::try_from_configs(
            u64::MAX,
            u64::MAX,
            ContractCostParams(vec![].try_into().unwrap()),
            ContractCostParams(vec![].try_into().unwrap()),
        )?;

        Ok(CoreCompilationContext { unlimited_budget })
    }
}

impl AsBudget for CoreCompilationContext {
    fn as_budget(&self) -> &Budget {
        &self.unlimited_budget
    }
}

impl ErrorHandler for CoreCompilationContext {
    fn map_err<T, E>(&self, res: Result<T, E>) -> Result<T, HostError>
    where
        soroban_env_host::Error: From<E>,
        E: core::fmt::Debug,
    {
        match res {
            Ok(t) => Ok(t),
            Err(e) => Err(HostError::from(e)),
        }
    }

    fn error(&self, error: soroban_env_host::Error, _msg: &str, _args: &[Val]) -> HostError {
        HostError::from(error)
    }
}
