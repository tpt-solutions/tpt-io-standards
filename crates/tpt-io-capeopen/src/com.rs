//! Windows-only FFI-safe wrappers around the CAPE-OPEN 1.1 / 2.0 thermo COM
//! interfaces.
//!
//! This module is compiled **only** with `--features com` on `cfg(windows)`. It
//! provides `#[repr(C)]` vtable definitions and safe Rust wrappers for the three
//! core CAPE-OPEN thermodynamic interfaces:
//!
//! * [`CapeThermoMaterial`] — `ICapeThermoMaterial` (material/stream state)
//! * [`CapeThermoCalculation`] — `ICapeThermoCalculation` (property calculations)
//! * [`CapeThermoPropertyPackage`] — `ICapeThermoPropertyPackage` (the package)
//!
//! The vtable layouts follow the published CAPE-OPEN Thermodynamic & Physical
//! Properties (1.1) IDL method order. **They must be audited against the
//! official type library before production use** — the method ordering is what
//! makes a `extern "system"` vtable call resolve to the correct COM method, and
//! this scaffold cannot verify it without the real CAPE-OPEN type library. The
//! Windows CI runner (`cargo build --features com`) checks that the code
//! compiles against the `windows` crate; it does not assert vtable correctness.
//!
//! All raw pointers are confined to this module; callers receive owned,
//! allocation-based Rust types ([`crate::model`]).

#![cfg(all(feature = "com", windows))]

use windows::core::{BSTR, GUID, HRESULT};
use windows::Win32::System::Com::{
    SafeArrayAccessData, SafeArrayDestroy, SafeArrayGetLBound, SafeArrayGetUBound, SafeArrayUnaccessData,
    SAFEARRAY,
};
use windows::Win32::System::Variant::{VariantToDouble, VT_BSTR, VT_R8};

use crate::error::CapeOpenError;
use crate::model::{ComponentList, Phase, PropertyPackage, StreamState};

type Raw = *mut core::ffi::c_void;

/// CAPE-OPEN 1.1 Thermo `ICapeThermoMaterial` IID.
///
/// **Unverified** — confirm against the published CAPE-OPEN type library.
pub const IID_ICAPE_THERMO_MATERIAL: GUID =
    GUID::from_u128(0xCF738632_0C79_48A1_9148_87CA96426637);

/// CAPE-OPEN 1.1 Thermo `ICapeThermoCalculation` IID. **Unverified.**
pub const IID_ICAPE_THERMO_CALCULATION: GUID =
    GUID::from_u128(0xCF738633_0C79_48A1_9148_87CA96426637);

/// CAPE-OPEN 1.1 Thermo `ICapeThermoPropertyPackage` IID. **Unverified.**
pub const IID_ICAPE_THERMO_PROPERTY_PACKAGE: GUID =
    GUID::from_u128(0xCF738634_0C79_48A1_9148_87CA96426637);

#[repr(C)]
struct IUnknownVtbl {
    query_interface: unsafe extern "system" fn(Raw, *const GUID, *mut Raw) -> HRESULT,
    add_ref: unsafe extern "system" fn(Raw) -> u32,
    release: unsafe extern "system" fn(Raw) -> u32,
}

#[repr(C)]
#[allow(clippy::missing_safety_doc)]
pub struct ICapeThermoMaterialVtbl {
    pub base: IUnknownVtbl,
    pub get_num_components: unsafe extern "system" fn(Raw, *mut i32) -> HRESULT,
    pub get_component_list: unsafe extern "system" fn(Raw, *mut *mut SAFEARRAY) -> HRESULT,
    pub get_component_id: unsafe extern "system" fn(Raw, i32, *mut BSTR) -> HRESULT,
    pub get_phase_list: unsafe extern "system" fn(Raw, *mut *mut SAFEARRAY) -> HRESULT,
    pub get_property: unsafe extern "system" fn(Raw, BSTR, BSTR, *mut windows::Win32::System::Variant::VARIANT) -> HRESULT,
    pub set_property: unsafe extern "system" fn(Raw, BSTR, BSTR, windows::Win32::System::Variant::VARIANT) -> HRESULT,
    pub get_temperature: unsafe extern "system" fn(Raw, *mut f64) -> HRESULT,
    pub set_temperature: unsafe extern "system" fn(Raw, f64) -> HRESULT,
    pub get_pressure: unsafe extern "system" fn(Raw, *mut f64) -> HRESULT,
    pub set_pressure: unsafe extern "system" fn(Raw, f64) -> HRESULT,
    pub get_overall_composition: unsafe extern "system" fn(Raw, *mut *mut SAFEARRAY) -> HRESULT,
    pub set_overall_composition: unsafe extern "system" fn(Raw, *mut SAFEARRAY) -> HRESULT,
}

#[repr(C)]
#[allow(clippy::missing_safety_doc)]
pub struct ICapeThermoCalculationVtbl {
    pub base: IUnknownVtbl,
    pub calc_single_phase_prop: unsafe extern "system" fn(Raw, BSTR, BSTR, *mut *mut SAFEARRAY) -> HRESULT,
    pub calc_two_phase_prop: unsafe extern "system" fn(Raw, BSTR, BSTR, BSTR, *mut *mut SAFEARRAY) -> HRESULT,
    pub calc_equilibrium: unsafe extern "system" fn(Raw, BSTR, BSTR, BSTR, *mut *mut SAFEARRAY) -> HRESULT,
}

#[repr(C)]
#[allow(clippy::missing_safety_doc)]
pub struct ICapeThermoPropertyPackageVtbl {
    pub base: IUnknownVtbl,
    pub get_component_list: unsafe extern "system" fn(Raw, *mut *mut SAFEARRAY) -> HRESULT,
    pub get_phase_list: unsafe extern "system" fn(Raw, *mut *mut SAFEARRAY) -> HRESULT,
    pub get_supported_single_phase_properties: unsafe extern "system" fn(Raw, *mut *mut SAFEARRAY) -> HRESULT,
    pub get_supported_two_phase_properties: unsafe extern "system" fn(Raw, *mut *mut SAFEARRAY) -> HRESULT,
}

fn hresult_to_error(hr: HRESULT, ctx: &str) -> CapeOpenError {
    if hr.is_ok() {
        CapeOpenError::Com(format!("{ctx}: unexpected success"))
    } else {
        CapeOpenError::Com(format!("{ctx}: COM error 0x{:08X}", hr.0))
    }
}

unsafe fn read_safearray_bstr(arr: *mut SAFEARRAY) -> Result<alloc::vec::Vec<String>, CapeOpenError> {
    if arr.is_null() {
        return Ok(alloc::vec::Vec::new());
    }
    let mut lbound: i32 = 0;
    let mut ubound: i32 = 0;
    SafeArrayGetLBound(arr, 1, &mut lbound)
        .ok()
        .map_err(|e| CapeOpenError::Com(format!("SafeArrayGetLBound: {e}")))?;
    SafeArrayGetUBound(arr, 1, &mut ubound)
        .ok()
        .map_err(|e| CapeOpenError::Com(format!("SafeArrayGetUBound: {e}")))?;
    let count = (ubound - lbound + 1) as usize;

    let mut data: *mut core::ffi::c_void = core::ptr::null_mut();
    SafeArrayAccessData(arr, &mut data)
        .ok()
        .map_err(|e| CapeOpenError::Com(format!("SafeArrayAccessData: {e}")))?;
    let elems = data as *mut BSTR;

    let mut out = alloc::vec::Vec::with_capacity(count);
    for k in 0..count {
        let b = *elems.add(k);
        out.push(String::from_utf16_lossy(b.as_wide()));
    }
    SafeArrayUnaccessData(arr)
        .ok()
        .map_err(|e| CapeOpenError::Com(format!("SafeArrayUnaccessData: {e}")))?;
    SafeArrayDestroy(arr)
        .ok()
        .map_err(|e| CapeOpenError::Com(format!("SafeArrayDestroy: {e}")))?;
    Ok(out)
}

unsafe fn read_safearray_f64(arr: *mut SAFEARRAY) -> Result<alloc::vec::Vec<f64>, CapeOpenError> {
    if arr.is_null() {
        return Ok(alloc::vec::Vec::new());
    }
    let mut lbound: i32 = 0;
    let mut ubound: i32 = 0;
    SafeArrayGetLBound(arr, 1, &mut lbound)
        .ok()
        .map_err(|e| CapeOpenError::Com(format!("SafeArrayGetLBound: {e}")))?;
    SafeArrayGetUBound(arr, 1, &mut ubound)
        .ok()
        .map_err(|e| CapeOpenError::Com(format!("SafeArrayGetUBound: {e}")))?;
    let count = (ubound - lbound + 1) as usize;

    let mut data: *mut core::ffi::c_void = core::ptr::null_mut();
    SafeArrayAccessData(arr, &mut data)
        .ok()
        .map_err(|e| CapeOpenError::Com(format!("SafeArrayAccessData: {e}")))?;
    let elems = data as *mut f64;

    let mut out = alloc::vec::Vec::with_capacity(count);
    for k in 0..count {
        out.push(*elems.add(k));
    }
    SafeArrayUnaccessData(arr)
        .ok()
        .map_err(|e| CapeOpenError::Com(format!("SafeArrayUnaccessData: {e}")))?;
    SafeArrayDestroy(arr)
        .ok()
        .map_err(|e| CapeOpenError::Com(format!("SafeArrayDestroy: {e}")))?;
    Ok(out)
}

/// Safe wrapper around an `ICapeThermoMaterial` COM interface pointer.
pub struct CapeThermoMaterial {
    ptr: *mut *mut ICapeThermoMaterialVtbl,
}

impl CapeThermoMaterial {
    /// Wrap a raw COM interface pointer. The caller must guarantee `ptr` points
    /// to a live `ICapeThermoMaterial` and that this wrapper takes ownership of
    /// one reference (it will `Release` on drop).
    ///
    /// # Safety
    /// `ptr` must be a valid, non-null `ICapeThermoMaterial` interface pointer.
    pub unsafe fn from_raw(ptr: Raw) -> Self {
        CapeThermoMaterial { ptr: ptr as *mut *mut ICapeThermoMaterialVtbl }
    }

    /// Borrow the raw interface pointer.
    pub fn as_raw(&self) -> Raw {
        self.ptr as Raw
    }

    /// Number of components in the material's component list.
    pub fn num_components(&self) -> Result<usize, CapeOpenError> {
        let mut n: i32 = 0;
        let hr = unsafe { ((*self.ptr).get_num_components)(self.ptr as Raw, &mut n) };
        if hr.is_err() {
            return Err(hresult_to_error(hr, "GetNumComponents"));
        }
        Ok(n.max(0) as usize)
    }

    /// Component ids (1-based `GetComponentID` enumeration).
    pub fn component_ids(&self) -> Result<alloc::vec::Vec<String>, CapeOpenError> {
        let n = self.num_components()?;
        let mut out = alloc::vec::Vec::with_capacity(n);
        for i in 1..=n as i32 {
            let mut b = BSTR::new();
            let hr = unsafe { ((*self.ptr).get_component_id)(self.ptr as Raw, i, &mut b) };
            if hr.is_err() {
                return Err(hresult_to_error(hr, "GetComponentID"));
            }
            out.push(String::from_utf16_lossy(b.as_wide()));
        }
        Ok(out)
    }

    /// Supported phases, as a [`ComponentList`]-free phase vector.
    pub fn phase_list(&self) -> Result<alloc::vec::Vec<Phase>, CapeOpenError> {
        let mut arr: *mut SAFEARRAY = core::ptr::null_mut();
        let hr = unsafe { ((*self.ptr).get_phase_list)(self.ptr as Raw, &mut arr) };
        if hr.is_err() {
            return Err(hresult_to_error(hr, "GetPhaseList"));
        }
        let names = unsafe { read_safearray_bstr(arr)? };
        Ok(names.into_iter().map(|s| Phase::parse(&s)).collect())
    }

    /// Read the material temperature (K).
    pub fn temperature(&self) -> Result<f64, CapeOpenError> {
        let mut t: f64 = 0.0;
        let hr = unsafe { ((*self.ptr).get_temperature)(self.ptr as Raw, &mut t) };
        if hr.is_err() {
            return Err(hresult_to_error(hr, "GetTemperature"));
        }
        Ok(t)
    }

    /// Read the material pressure (Pa).
    pub fn pressure(&self) -> Result<f64, CapeOpenError> {
        let mut p: f64 = 0.0;
        let hr = unsafe { ((*self.ptr).get_pressure)(self.ptr as Raw, &mut p) };
        if hr.is_err() {
            return Err(hresult_to_error(hr, "GetPressure"));
        }
        Ok(p)
    }

    /// Read overall mole fractions (indexed by component order).
    pub fn overall_composition(&self) -> Result<alloc::vec::Vec<f64>, CapeOpenError> {
        let mut arr: *mut SAFEARRAY = core::ptr::null_mut();
        let hr = unsafe { ((*self.ptr).get_overall_composition)(self.ptr as Raw, &mut arr) };
        if hr.is_err() {
            return Err(hresult_to_error(hr, "GetOverallComposition"));
        }
        unsafe { read_safearray_f64(arr) }
    }

    /// Read a single scalar property for a phase via `GetProperty`.
    pub fn property(&self, phase: Phase, property: &str) -> Result<f64, CapeOpenError> {
        let phase_b = BSTR::from(phase.as_str());
        let prop_b = BSTR::from(property);
        let mut var = windows::Win32::System::Variant::VARIANT::default();
        let hr = unsafe { ((*self.ptr).get_property)(self.ptr as Raw, phase_b, prop_b, &mut var) };
        if hr.is_err() {
            return Err(hresult_to_error(hr, "GetProperty"));
        }
        let mut v: f64 = 0.0;
        VariantToDouble(&var, &mut v)
            .ok()
            .map_err(|e| CapeOpenError::Com(format!("VariantToDouble: {e}")))?;
        Ok(v)
    }

    /// Pull the full material state into the pure-Rust [`StreamState`].
    pub fn to_stream_state(&self, n_components: usize) -> Result<StreamState, CapeOpenError> {
        let temperature = self.temperature()?;
        let pressure = self.pressure()?;
        let overall_mole_fractions = self.overall_composition()?;
        if overall_mole_fractions.len() != n_components {
            return Err(CapeOpenError::Com(format!(
                "composition has {} entries, expected {n_components}",
                overall_mole_fractions.len()
            )));
        }
        Ok(StreamState { temperature, pressure, overall_mole_fractions, total_flow: 0.0 })
    }
}

impl Drop for CapeThermoMaterial {
    fn drop(&mut self) {
        unsafe {
            ((*self.ptr).base.release)(self.ptr as Raw);
        }
    }
}

/// Safe wrapper around an `ICapeThermoCalculation` COM interface pointer.
pub struct CapeThermoCalculation {
    ptr: *mut *mut ICapeThermoCalculationVtbl,
}

impl CapeThermoCalculation {
    /// Wrap a raw `ICapeThermoCalculation` interface pointer.
    ///
    /// # Safety
    /// `ptr` must be a valid, non-null `ICapeThermoCalculation` interface pointer.
    pub unsafe fn from_raw(ptr: Raw) -> Self {
        CapeThermoCalculation { ptr: ptr as *mut *mut ICapeThermoCalculationVtbl }
    }

    /// `CalcSinglePhaseProp` — compute `property` for `phase`, returning a flat
    /// result vector (per-component, in component-list order).
    pub fn calc_single_phase_prop(
        &self,
        property: &str,
        phase: &str,
    ) -> Result<alloc::vec::Vec<f64>, CapeOpenError> {
        let prop_b = BSTR::from(property);
        let phase_b = BSTR::from(phase);
        let mut arr: *mut SAFEARRAY = core::ptr::null_mut();
        let hr =
            unsafe { ((*self.ptr).calc_single_phase_prop)(self.ptr as Raw, prop_b, phase_b, &mut arr) };
        if hr.is_err() {
            return Err(hresult_to_error(hr, "CalcSinglePhaseProp"));
        }
        unsafe { read_safearray_f64(arr) }
    }

    /// `CalcEquilibrium` — flash for `pressure`/`temperature`/`flashCondition`.
    pub fn calc_equilibrium(
        &self,
        pressure: &str,
        temperature: &str,
        flash_condition: &str,
    ) -> Result<alloc::vec::Vec<f64>, CapeOpenError> {
        let p = BSTR::from(pressure);
        let t = BSTR::from(temperature);
        let f = BSTR::from(flash_condition);
        let mut arr: *mut SAFEARRAY = core::ptr::null_mut();
        let hr = unsafe { ((*self.ptr).calc_equilibrium)(self.ptr as Raw, p, t, f, &mut arr) };
        if hr.is_err() {
            return Err(hresult_to_error(hr, "CalcEquilibrium"));
        }
        unsafe { read_safearray_f64(arr) }
    }
}

impl Drop for CapeThermoCalculation {
    fn drop(&mut self) {
        unsafe {
            ((*self.ptr).base.release)(self.ptr as Raw);
        }
    }
}

/// Safe wrapper around an `ICapeThermoPropertyPackage` COM interface pointer.
pub struct CapeThermoPropertyPackage {
    ptr: *mut *mut ICapeThermoPropertyPackageVtbl,
}

impl CapeThermoPropertyPackage {
    /// Wrap a raw `ICapeThermoPropertyPackage` interface pointer.
    ///
    /// # Safety
    /// `ptr` must be a valid, non-null `ICapeThermoPropertyPackage` interface
    /// pointer.
    pub unsafe fn from_raw(ptr: Raw) -> Self {
        CapeThermoPropertyPackage { ptr: ptr as *mut *mut ICapeThermoPropertyPackageVtbl }
    }

    /// Pull the package metadata into the pure-Rust [`PropertyPackage`].
    pub fn to_property_package(&self, name: &str) -> Result<PropertyPackage, CapeOpenError> {
        let mut arr: *mut SAFEARRAY = core::ptr::null_mut();
        let hr = unsafe { ((*self.ptr).get_component_list)(self.ptr as Raw, &mut arr) };
        if hr.is_err() {
            return Err(hresult_to_error(hr, "GetComponentList"));
        }
        let ids = unsafe { read_safearray_bstr(arr)? };
        let components = ComponentList { components: ids.into_iter().map(|id| crate::model::Component::new(id)).collect() };

        let mut parr: *mut SAFEARRAY = core::ptr::null_mut();
        let hr = unsafe { ((*self.ptr).get_phase_list)(self.ptr as Raw, &mut parr) };
        if hr.is_err() {
            return Err(hresult_to_error(hr, "GetPhaseList"));
        }
        let phases = unsafe { read_safearray_bstr(parr)? }
            .into_iter()
            .map(|s| Phase::parse(&s))
            .collect();

        let mut sarr: *mut SAFEARRAY = core::ptr::null_mut();
        let hr = unsafe {
            (*self.ptr).get_supported_single_phase_properties(self.ptr as Raw, &mut sarr)
        };
        if hr.is_err() {
            return Err(hresult_to_error(hr, "GetSupportedSinglePhaseProperties"));
        }
        let single = unsafe { read_safearray_bstr(sarr)? }
            .into_iter()
            .map(|s| crate::model::BaseProperty::parse(&s))
            .collect();

        let mut tarr: *mut SAFEARRAY = core::ptr::null_mut();
        let hr = unsafe {
            (*self.ptr).get_supported_two_phase_properties(self.ptr as Raw, &mut tarr)
        };
        if hr.is_err() {
            return Err(hresult_to_error(hr, "GetSupportedTwoPhaseProperties"));
        }
        let two = unsafe { read_safearray_bstr(tarr)? }
            .into_iter()
            .map(|s| crate::model::BaseProperty::parse(&s))
            .collect();

        let pp = PropertyPackage { name: name.to_string(), components, supported_phases: phases, supported_single_phase: single, supported_two_phase: two };
        pp.validate().map_err(CapeOpenError::PropertyPackage)?;
        Ok(pp)
    }
}

impl Drop for CapeThermoPropertyPackage {
    fn drop(&mut self) {
        unsafe {
            ((*self.ptr).base.release)(self.ptr as Raw);
        }
    }
}

// Keep `VT_BSTR` / `VT_R8` referenced so the imports are meaningful even if a
// future refactor drops their only use site.
const _: u16 = VT_BSTR.0;
const _: u16 = VT_R8.0;
