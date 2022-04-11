//
// Copyright (c) 2022 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//

use async_trait::async_trait;
use pyo3::{prelude::*, types::PyModule};
use std::fs;
use std::path::Path;
use zenoh_flow::async_std::sync::Arc;
use zenoh_flow::Configuration;
use zenoh_flow::{Data, Node, Source, State, ZFError, ZFResult};
use zenoh_flow_python_common::configuration_into_py;
use zenoh_flow_python_common::PythonState;
use zenoh_flow_python_common::{from_pyany_to_data, from_pyerr_to_zferr};

#[cfg(target_family = "unix")]
use libloading::os::unix::Library;
#[cfg(target_family = "windows")]
use libloading::Library;

#[cfg(target_family = "unix")]
static LOAD_FLAGS: std::os::raw::c_int =
    libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL;

pub static PY_LIB: &str = env!("PY_LIB");

#[derive(Debug)]
struct PySource(Library);

#[async_trait]
impl Source for PySource {
    async fn run(&self, ctx: &mut zenoh_flow::Context, state: &mut State) -> ZFResult<Data> {
        // Preparing python
        let gil = Python::acquire_gil();
        let py = gil.python();

        // Preparing parameter
        let current_state = state.try_get::<PythonState>()?;
        let source_class = current_state.module.as_ref().clone();
        let zf_types_module = current_state
            .py_zf_types
            .cast_as::<PyModule>(py)
            .map_err(|e| from_pyerr_to_zferr(e.into(), &py))?;

        let py_ctx = zenoh_flow_python_common::from_context_to_pyany(ctx, &py, zf_types_module)?;

        let py_value = source_class
            .call_method1(
                py,
                "run",
                (
                    source_class.clone(),
                    py_ctx,
                    current_state.py_state.as_ref().clone(),
                ),
            )
            .map_err(|e| from_pyerr_to_zferr(e, &py))?
            .into_ref(py);

        // Calling python
        let value = from_pyany_to_data(py_value, &py)?;

        // Converting to rust types
        Ok(value)
    }
}

impl Node for PySource {
    fn initialize(&self, configuration: &Option<Configuration>) -> ZFResult<State> {
        // Preparing python
        pyo3::prepare_freethreaded_python();
        let gil = Python::acquire_gil();
        let py = gil.python();

        // Configuring wrapper + python source
        match configuration {
            Some(configuration) => {
                // Unwrapping configuration
                let script_file_path = Path::new(
                    configuration["python-script"]
                        .as_str()
                        .ok_or(ZFError::InvalidState)?,
                );
                let mut config = configuration.clone();

                config["python-script"].take();
                let py_config = config["configuration"].take();

                // Convert configuration to Python
                let py_config = configuration_into_py(py, py_config)
                    .map_err(|e| from_pyerr_to_zferr(e, &py))?;

                let py_zf_types = PyModule::import(py, "zenoh_flow.types")
                    .map_err(|e| from_pyerr_to_zferr(e, &py))?
                    .to_object(py);

                // Load Python code
                let code = read_file(script_file_path)?;
                let module =
                    PyModule::from_code(py, &code, &script_file_path.to_string_lossy(), "source")
                        .map_err(|e| from_pyerr_to_zferr(e, &py))?;
                // Getting the correct python module
                let source_class: PyObject = module
                    .call_method0("register")
                    .map_err(|e| from_pyerr_to_zferr(e, &py))?
                    .into();

                // Initialize python state
                let state: PyObject = source_class
                    .call_method1(py, "initialize", (source_class.clone(), py_config))
                    .map_err(|e| from_pyerr_to_zferr(e, &py))?;

                Ok(State::from(PythonState {
                    module: Arc::new(source_class),
                    py_state: Arc::new(state),
                    py_zf_types: Arc::new(py_zf_types),
                }))
            }
            None => Err(ZFError::InvalidState),
        }
    }

    fn finalize(&self, state: &mut State) -> ZFResult<()> {
        let gil = Python::acquire_gil();
        let py = gil.python();
        let current_state = state.try_get::<PythonState>()?;
        let src_class = current_state.module.as_ref().clone();

        src_class
            .call_method1(
                py,
                "finalize",
                (src_class.clone(), current_state.py_state.as_ref().clone()),
            )
            .map_err(|e| from_pyerr_to_zferr(e, &py))?;

        Ok(())
    }
}

// Also generated by macro
zenoh_flow::export_source!(register);

fn load_self() -> ZFResult<Library> {
    log::trace!("Python Source Wrapper loading Python {}", PY_LIB);

    // Very dirty hack!
    let lib_name = libloading::library_filename(PY_LIB);
    unsafe {
        #[cfg(target_family = "unix")]
        let lib = Library::open(Some(lib_name), LOAD_FLAGS)?;

        #[cfg(target_family = "windows")]
        let lib = Library::new(lib_name)?;

        Ok(lib)
    }
}
fn register() -> ZFResult<Arc<dyn Source>> {
    let library = load_self()?;

    Ok(Arc::new(PySource(library)) as Arc<dyn Source>)
}

fn read_file(path: &Path) -> ZFResult<String> {
    Ok(fs::read_to_string(path)?)
}
