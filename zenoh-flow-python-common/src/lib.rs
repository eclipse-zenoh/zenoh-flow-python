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

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyLong, PyString, PyTuple};
use std::convert::{TryFrom, TryInto};
use zenoh_flow::bail;

use zenoh_flow::prelude::{
    zferror, Configuration, Context as ZFContext, Data, Error, ErrorKind, Input,
    Message as ZFMessage, Output, PortId,
};

use std::sync::Arc;

#[derive(Clone)]
pub struct PythonState {
    pub module: Arc<PyObject>,
    pub py_state: Arc<PyObject>,
    pub event_loop: Arc<PyObject>,
    pub asyncio_module: Arc<PyObject>,
}

impl Drop for PythonState {
    fn drop(&mut self) {
        let gil = Python::acquire_gil();
        let py = gil.python();

        let py_op = self
            .module
            .cast_as::<PyAny>(py)
            .expect("Unable to get Python Node module!");

        py_op
            .call_method1("finalize", (py_op,))
            .expect("Unable to call Python finalize!");
    }
}

unsafe impl Send for PythonState {}
unsafe impl Sync for PythonState {}

impl std::fmt::Debug for PythonState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PythonState").finish()
    }
}

pub fn from_pyerr_to_zferr(py_err: pyo3::PyErr, py: &pyo3::Python<'_>) -> Error {
    let tb = if let Some(traceback) = py_err.traceback(*py) {
        traceback.format().map_or_else(|_| "".to_string(), |s| s)
    } else {
        "".to_string()
    };

    zferror!(
        ErrorKind::InvalidData,
        "Error: {:?}\nTraceback: {:?}",
        py_err,
        tb
    )
    .into()
}

pub fn configuration_into_py(py: Python, value: Configuration) -> PyResult<PyObject> {
    match value {
        Configuration::Array(arr) => {
            let py_list = PyList::empty(py);
            for v in arr {
                py_list.append(configuration_into_py(py, v)?)?;
            }
            Ok(py_list.to_object(py))
        }
        Configuration::Object(obj) => {
            let py_dict = PyDict::new(py);
            for (k, v) in obj {
                py_dict.set_item(k, configuration_into_py(py, v)?)?;
            }
            Ok(py_dict.to_object(py))
        }
        Configuration::Bool(b) => Ok(b.to_object(py)),
        Configuration::Number(n) => {
            if n.is_i64() {
                Ok(n.as_i64()
                    .ok_or_else(|| {
                        PyErr::from_value(
                            PyTypeError::new_err(format!("Unable to convert {:?} to i64", n))
                                .value(py),
                        )
                    })?
                    .to_object(py))
            } else if n.is_u64() {
                Ok(n.as_u64()
                    .ok_or_else(|| {
                        PyErr::from_value(
                            PyTypeError::new_err(format!("Unable to convert {:?} to u64", n))
                                .value(py),
                        )
                    })?
                    .to_object(py))
            } else {
                Ok(n.as_f64()
                    .ok_or_else(|| {
                        PyErr::from_value(
                            PyTypeError::new_err(format!("Unable to convert {:?} to f64", n))
                                .value(py),
                        )
                    })?
                    .to_object(py))
            }
        }
        Configuration::String(s) => Ok(s.to_object(py)),
        Configuration::Null => Ok(py.None()),
    }
}

// pub fn register_python_callbacks(
//     ctx: &mut ZFContext,
//     py_ctx: Context,
//     py_receivers: &PyDict,
//     py_senders: &PyDict,
//     py: Python,
// ) -> PyResult<()> {
//     let (mut cb_senders, mut cb_receivers) = py_ctx.split();

//     // Converting receivers callbacks
//     for (id, cb, receiver) in cb_receivers.drain(..) {
//         match py_receivers.del_item(PyString::new(py, &id)) {
//             Ok(_) => {
//                 // This should have dropped the only reference to the Arc<Input>

//                 // So we can get back an owned Input
//                 match Arc::try_unwrap(receiver) {
//                                     Ok(input) => {
//                                         // If it is ok, we can now create the callback.
//                                         input.into_callback(ctx, Box::new( move |data: zenoh_flow::prelude::Message | {
//                                             let c_data = data.clone();
//                                             let c_cb = cb.clone();

//                                             async move {
//                                                 let py_msg = DataMessage::try_from(c_data)?;

//                                                 // Python callbacks are simple
//                                                 // lambda functions,
//                                                 // no awaitable can be used.

//                                                 Python::with_gil(|py| {
//                                                     c_cb.call1(py, (py_msg, ))
//                                                 })
//                                                 .map_err(|e| Python::with_gil(|py| from_pyerr_to_zferr(e, &py)))?;
//                                                 Ok(())
//                                             }
//                                         }
//                                         ));

//                                     },
//                                     Err(_) => return Err(PyTypeError::new_err(format!("Cannot get Input from Python, maybe using a callback in the iteration function?")))
//                                 }
//             }
//             Err(_) => {
//                 return Err(PyTypeError::new_err(format!(
//                     "Cannot find {} in Python Receivers dictionary",
//                     id
//                 )))
//             }
//         }
//     }

//     // Converting senders callbacks
//     for (id, cb, sender) in cb_senders.drain(..) {
//         match py_senders.del_item(PyString::new(py, &id)) {
//             Ok(_) => {
//                 // This should have dropped the only reference to the Arc<Input>

//                 // So we can get back an owned Input
//                 match Arc::try_unwrap(sender) {
//                                     Ok(output) => {
//                                         // If it is ok, we can now create the callback.
//                                         output.into_callback(ctx, Box::new( move || {
//                                             let c_cb = cb.clone();
//                                             async move {
//                                                 // Python callbacks are simple
//                                                 // lambda functions,
//                                                 // no awaitable can be used.
//                                                 let (data, ts) = Python::with_gil(|py| {
//                                                     let res = c_cb.call0(py)?;

//                                                     // let py_tuple_res : PyTuple = res.cast_as(py)?;
//                                                     let py_data : &PyBytes = res.cast_as(py)?;

//                                                     let rust_data = Data::from(py_data.as_bytes());
//                                                     Ok((rust_data, None))

//                                                 }).map_err(|e| Python::with_gil(|py| from_pyerr_to_zferr(e, &py)))?;

//                                                 Ok((data, ts)) }
//                                         }
//                                         ));

//                                     },
//                                     Err(_) => return Err(PyTypeError::new_err(format!("Cannot get Output from Python, maybe using a callback in the iteration function?")))
//                                 }
//             }
//             Err(_) => {
//                 return Err(PyTypeError::new_err(format!(
//                     "Cannot find {} in Python Senders dictionary",
//                     id
//                 )))
//             }
//         }
//     }
//     Ok(())
// }

// /// The Python context
// /// It contains a two vectors, with respecively the
// /// callbacks in senders and in receivers.
// #[pyclass]
// pub struct Context {
//     pub(crate) callback_senders: Vec<(PortId, Py<PyAny>, Arc<Output>)>,
//     pub(crate) callback_receivers: Vec<(PortId, Py<PyAny>, Arc<Input>)>,
// }

// #[pymethods]
// impl Context {
//     #[new]
//     pub fn new() -> Self {
//         Self {
//             callback_receivers: Vec::new(),
//             callback_senders: Vec::new(),
//         }
//     }
// }

// impl Context {
//     pub fn split(
//         self,
//     ) -> (
//         Vec<(PortId, Py<PyAny>, Arc<Output>)>,
//         Vec<(PortId, Py<PyAny>, Arc<Input>)>,
//     ) {
//         (self.callback_senders, self.callback_receivers)
//     }
// }

#[pyclass]
pub struct DataSender {
    pub(crate) sender: Arc<Output>,
}

// unsafe impl Send for DataSender {}
// unsafe impl Sync for DataSender {}

#[pymethods]
impl DataSender {
    pub fn send<'p>(
        &'p self,
        py: Python<'p>,
        data: &PyBytes,
        ts: Option<u64>,
    ) -> PyResult<&'p PyAny> {
        let c_sender = self.sender.clone();
        let rust_data = Data::from(data.as_bytes());
        pyo3_asyncio::async_std::future_into_py(py, async move {
            c_sender
                .send_async(rust_data, ts)
                .await
                .map_err(|_| PyValueError::new_err("Unable to send data"))?;
            Ok(Python::with_gil(|py| py.None()))
        })
    }

    // pub fn into_callback<'p>(&'p self, ctx: &mut Context, cb: Py<PyAny>) -> PyResult<()> {
    //     ctx.callback_senders
    //         .push((self.sender.port_id().clone(), cb, self.sender.clone()));
    //     Ok(())
    // }
}

impl From<Output> for DataSender {
    fn from(other: Output) -> Self {
        Self {
            sender: Arc::new(other),
        }
    }
}

#[pyclass(subclass)]
pub struct DataReceiver {
    pub(crate) receiver: Arc<Input>,
}

// unsafe impl Send for DataReceiver {}
// unsafe impl Sync for DataReceiver {}

#[pymethods]
impl DataReceiver {
    pub fn recv<'p>(&'p self, py: Python<'p>) -> PyResult<&'p PyAny> {
        let c_receiver = self.receiver.clone();
        pyo3_asyncio::async_std::future_into_py(py, async move {
            let rust_msg = c_receiver
                .recv_async()
                .await
                .map_err(|_| PyValueError::new_err("Unable to receive data"))?;
            DataMessage::try_from(rust_msg)
        })
    }

    // pub fn into_callback<'p>(&'p self, ctx: &mut Context, cb: Py<PyAny>) -> PyResult<()> {
    //     ctx.callback_receivers
    //         .push((self.receiver.id().clone(), cb, self.receiver.clone()));
    //     Ok(())
    // }
}

impl From<Input> for DataReceiver {
    fn from(other: Input) -> Self {
        Self {
            receiver: Arc::new(other),
        }
    }
}

impl TryInto<Input> for DataReceiver {
    type Error = zenoh_flow::prelude::Error;

    fn try_into(self) -> Result<Input, Self::Error> {
        match Arc::try_unwrap(self.receiver) {
            Ok(input) => Ok(input),
            Err(_) => bail!(
                ErrorKind::GenericError,
                "Cannot get Input from Python, maybe using a callback in the iteration function?"
            ),
        }
    }
}

#[pyclass(subclass)]
pub struct DataMessage {
    data: Py<PyBytes>,
    ts: Py<PyLong>,
    is_watermark: bool,
}

#[pymethods]
impl DataMessage {
    #[new]
    pub fn new(data: Py<PyBytes>, ts: Py<PyLong>, is_watermark: bool) -> Self {
        Self {
            data,
            ts,
            is_watermark,
        }
    }

    #[getter]
    pub fn get_data(&self) -> &Py<PyBytes> {
        &self.data
    }

    #[getter]
    pub fn get_ts(&self) -> &Py<PyLong> {
        &self.ts
    }

    #[getter]
    pub fn is_watermark(&self) -> bool {
        self.is_watermark
    }
}

impl TryFrom<ZFMessage> for DataMessage {
    type Error = PyErr;

    fn try_from(other: ZFMessage) -> Result<Self, Self::Error> {
        match other {
            ZFMessage::Data(mut msg) => {
                let data = Python::with_gil(|py| {
                    let bytes = msg
                        .get_inner_data()
                        .try_as_bytes()
                        .map_err(|e| PyValueError::new_err(format!("try_as_bytes field: {e}")))?;

                    Ok::<pyo3::Py<PyBytes>, Self::Error>(Py::from(PyBytes::new(py, bytes.as_ref())))
                })?;

                let ts: Py<PyLong> = Python::with_gil(|py| {
                    Ok::<pyo3::Py<PyLong>, Self::Error>(Py::from(
                        msg.get_timestamp()
                            .get_time()
                            .as_u64()
                            .to_object(py)
                            .cast_as::<PyLong>(py)?,
                    ))
                })?;

                Ok(Self {
                    data,
                    ts,
                    is_watermark: false,
                })
            }
            ZFMessage::Watermark(ts) => {
                let data = Python::with_gil(|py| Py::from(PyBytes::new(py, &[0u8])));
                let ts = Python::with_gil(|py| {
                    Ok::<pyo3::Py<PyLong>, Self::Error>(Py::from(
                        ts.get_time()
                            .as_u64()
                            .to_object(py)
                            .cast_as::<PyLong>(py)
                            .unwrap(),
                    ))
                })?;

                Ok(Self {
                    data,
                    ts,
                    is_watermark: true,
                })
            }
            _ => Err(PyValueError::new_err(
                "Cannot convert ControlMessage to DataMessage",
            )),
        }
    }
}
