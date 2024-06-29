#[macro_use]
extern crate serde; // Import the serde library for serialization and deserialization

use candid::{Decode, Encode}; // Import Decode and Encode from the candid library
use ic_cdk::api::time; // Import the time API from ic_cdk
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory}; // Import memory management structures from ic_stable_structures
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable}; // Import stable structures
use std::{borrow::Cow, cell::RefCell}; // Import Cow and RefCell from the standard library

type Memory = VirtualMemory<DefaultMemoryImpl>; // Type alias for VirtualMemory using DefaultMemoryImpl
type IdCell = Cell<u64, Memory>; // Type alias for Cell storing u64 with Memory

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)] // Derive macros for the Mahasiswa struct
struct Mahasiswa {
    id: u64, // Unique identifier for the Mahasiswa
    nama: String, // Name of the Mahasiswa
    nim: String, // NIM of the Mahasiswa
    jurusan: String, // Jurusan of the Mahasiswa
    angkatan: u64, // Angkatan of the Mahasiswa
    created_at: u64, // Timestamp of when the Mahasiswa was created
    updated_at: Option<u64>, // Optional timestamp of when the Mahasiswa was last updated
}

// Implement the Storable trait for the Mahasiswa struct
impl Storable for Mahasiswa {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap()) // Serialize the Mahasiswa struct to bytes
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap() // Deserialize bytes to a Mahasiswa struct
    }
}

// Implement the BoundedStorable trait for the Mahasiswa struct
impl BoundedStorable for Mahasiswa {
    const MAX_SIZE: u32 = 1024; // Maximum size of the serialized Mahasiswa in bytes
    const IS_FIXED_SIZE: bool = false; // Indicates that the size is not fixed
}

thread_local! {
    // Thread-local storage for memory manager
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    // Thread-local storage for ID counter
    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Tidak dapat membuat counter")
    );

    // Thread-local storage for the Mahasiswa storage
    static STORAGE: RefCell<StableBTreeMap<u64, Mahasiswa, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)] // Derive macros for the MahasiswaPayload struct
struct MahasiswaPayload {
    nama: String, // Name of the Mahasiswa
    nim: String, // NIM of the Mahasiswa
    jurusan: String, // Jurusan of the Mahasiswa
    angkatan: u64, // Angkatan of the Mahasiswa
}

#[ic_cdk::query] // Mark the function as a query method
fn get_mahasiswa(id: u64) -> Result<Mahasiswa, Error> {
    match _get_mahasiswa(&id) {
        Some(mahasiswa) => Ok(mahasiswa), // Return the Mahasiswa if found
        None => Err(Error::NotFound {
            msg: format!("Mahasiswa dengan id={} tidak ditemukan", id), // Return an error if the Mahasiswa is not found
        }),
    }
}

#[ic_cdk::update] // Mark the function as an update method
fn add_mahasiswa(payload: MahasiswaPayload) -> Result<Mahasiswa, Error> {
    if payload.nama.is_empty() || payload.nim.is_empty() || payload.jurusan.is_empty() {
        return Err(Error::InvalidInput { msg: "All fields must be provided and non-empty".to_string() });
    }

    // Increment the ID counter
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("Tidak dapat mengincrement id counter");

    // Create a new Mahasiswa struct
    let mahasiswa = Mahasiswa {
        id,
        nama: payload.nama,
        nim: payload.nim,
        jurusan: payload.jurusan,
        angkatan: payload.angkatan,
        created_at: time(),
        updated_at: None,
    };

    // Insert the new Mahasiswa into storage
    do_insert(&mahasiswa);

    Ok(mahasiswa)
}

#[ic_cdk::update] // Mark the function as an update method
fn update_mahasiswa(id: u64, payload: MahasiswaPayload) -> Result<Mahasiswa, Error> {
    if payload.nama.is_empty() || payload.nim.is_empty() || payload.jurusan.is_empty() {
        return Err(Error::InvalidInput { msg: "All fields must be provided and non-empty".to_string() });
    }

    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut mahasiswa) => {
            mahasiswa.nama = payload.nama;
            mahasiswa.nim = payload.nim;
            mahasiswa.jurusan = payload.jurusan;
            mahasiswa.angkatan = payload.angkatan;
            mahasiswa.updated_at = Some(time());

            // Update the Mahasiswa in storage
            do_insert(&mahasiswa);

            Ok(mahasiswa)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "Tidak dapat mengupdate mahasiswa dengan id={}. Mahasiswa tidak ditemukan",
                id
            ),
        }),
    }
}

// Helper method to perform insert operation
fn do_insert(mahasiswa: &Mahasiswa) {
    STORAGE.with(|service| service.borrow_mut().insert(mahasiswa.id, mahasiswa.clone()));
}

#[ic_cdk::update] // Mark the function as an update method
fn delete_mahasiswa(id: u64) -> Result<Mahasiswa, Error> {
    match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(mahasiswa) => Ok(mahasiswa), // Return the deleted Mahasiswa if found
        None => Err(Error::NotFound {
            msg: format!(
                "Tidak dapat menghapus mahasiswa dengan id={}. Mahasiswa tidak ditemukan.",
                id
            ),
        }),
    }
}

#[ic_cdk::query] // Mark the function as a query method
fn list_mahasiswa() -> Vec<Mahasiswa> {
    STORAGE.with(|service| {
        service.borrow().iter().map(|(_, mahasiswa)| mahasiswa).collect() // Return a list of all Mahasiswa
    })
}

#[ic_cdk::query] // Mark the function as a query method
fn find_mahasiswa_by_name(nama: String) -> Vec<Mahasiswa> {
    STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .filter(|(_, mahasiswa)| mahasiswa.nama.to_lowercase().contains(&nama.to_lowercase())) // Filter Mahasiswa by name
            .map(|(_, mahasiswa)| mahasiswa)
            .collect()
    })
}

#[ic_cdk::query] // Mark the function as a query method
fn find_mahasiswa_by_nim(nim: String) -> Option<Mahasiswa> {
    STORAGE.with(|service| {
        service
            .borrow()
            .iter()
            .find(|(_, mahasiswa)| mahasiswa.nim == nim) // Find Mahasiswa by NIM
            .map(|(_, mahasiswa)| mahasiswa)
    })
}

#[derive(candid::CandidType, Deserialize, Serialize)] // Derive macros for the Error enum
enum Error {
    NotFound { msg: String }, // Error variant for not found
    InvalidInput { msg: String }, // Error variant for invalid input
}

// Helper method to get a Mahasiswa by ID, used in get_mahasiswa and update_mahasiswa
fn _get_mahasiswa(id: &u64) -> Option<Mahasiswa> {
    STORAGE.with(|service| service.borrow().get(id))
}

// Generate candid interface
ic_cdk::export_candid!();
