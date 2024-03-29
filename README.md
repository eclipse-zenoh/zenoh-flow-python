[![Discussion](https://img.shields.io/badge/discussion-on%20github-blue)](https://github.com/eclipse-zenoh/roadmap/discussions)
[![Discord](https://img.shields.io/badge/chat-on%20discord-blue)](https://discord.gg/vSDSpqnbkm)
[![Eclipse CI](https://ci.eclipse.org/zenoh/buildStatus/icon?job=zenoh-flow-python-nightly&subject=Eclipse%20CI)](https://ci.eclipse.org/zenoh/view/Zenoh%20Flow/job/zenoh-flow-python-nightly/)
[![CI](https://github.com/eclipse-zenoh/zenoh-flow-python/actions/workflows/ci.yml/badge.svg)](https://github.com/eclipse-zenoh/zenoh-flow-python/actions/workflows/ci.yml)

<img src="https://raw.githubusercontent.com/eclipse-zenoh/zenoh/master/zenoh-dragon.png" height="150">


# Python Zenoh Flow API
[Zenoh Flow](https://github.com/eclipse-zenoh/zenoh-flow) provides a Zenoh-based dataflow programming framework for computations that span from the cloud to the device.

:warning: **This software is still in alpha status and should _not_ be used in production. Breaking changes are likely to happen and the API is not stable.**

-----------

### Requirements

- Rust: see the [installation page](https://www.rust-lang.org/tools/install)
- a matching version of libpython. On linux systems, it's typically packaged separately as ``libpython3.x-dev` or `python3.x-dev`.
- Python >= 3.7
- pip >= 22
- virtualenv




### How to build

Create and activate a python virtual environment:

```bash
$ python3 -m virtualenv venv
$ source venv/bin/activate
```

Build the Python Wheel **within** a Python virtual environment.

```bash
$ python3 -m venv venv
$ source venv/bin/activate
(venv) $ git clone https://github.com/eclipse-zenoh/zenoh-flow-python
(venv) $ cd zenoh-flow-python/zenoh-flow-python
(venv) $ pip3 install -r requirements-dev.txt
(venv) $ maturin build --release
```

**Deactivate** the venv and install the python bindings.

```bash
(venv) deactivate
$ pip3 install ./target/wheels/<there should only be one .whl file here>
```

#### Build the wrappers

Build the Python wrappers.

:warning: **Python Wrappers SHOULD NOT be built within a Python virtual environment**

```bash
$ cargo build --release -p zenoh-flow-python-operator-wrapper -p zenoh-flow-python-sink-wrapper -p zenoh-flow-python-source-wrapper
```

#### Build the docs

Once you have installed the Python binding you can also generate the documentation.
```
$ cd docs
$ pip3 install sphinx_rtd_theme sphinx -y
$ make html
```

The docs will be available under `_build/html/index.html`.


#### Install the Python extension for Zenoh-Flow

In order to install the Python extension, please execute the following steps:
- Copy the result of the build into `/var/zenoh-flow/python/`: `sudo cp ./target/release/libzenoh_flow_python* /var/zenoh-flow/python/`
- Update the paths in `01-python.zfext` according to your operating system (i.e., replace `.so` with `.dylib` for MacOS).
- Copy `01-python.zfext` into `/etc/zenoh-flow/extensions.d`: `sudo cp 01-python.zfext /etc/zenoh-flow/extensions.d/`


### Run an example

Please refer to our getting started example: [Zenoh-Flow Getting started](https://github.com/ZettaScaleLabs/zenoh-flow-examples/tree/master/getting-started).


