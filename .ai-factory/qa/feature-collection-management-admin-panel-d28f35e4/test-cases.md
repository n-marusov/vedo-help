## Test Cases: Collection Management in Admin Panel

> TDD RED phase — tests are written before implementation. All tests will fail initially and pass after the corresponding GREEN implementation phase.

---

### TC-001: Create a collection and verify it appears in the list

**Priority:** High
**Type:** Positive

**Precondition:**
In-memory SQLite with collections table migrated.

**Steps:**
1. Create a `CollectionRepository` with in-memory SQLite
2. Insert a collection with name "Test Collection" and description "A test"
3. Call `list_collections()` to retrieve all collections
4. Assert the returned list contains exactly 1 collection with name "Test Collection"

**Expected result:**
The collection is created and appears in the list with the correct name and description.

**Test data:**
```
name: "Test Collection"
description: "A test"
```

---

### TC-002: List collections returns empty list when no collections exist

**Priority:** Medium
**Type:** Positive

**Precondition:**
In-memory SQLite with collections table migrated, no collections inserted.

**Steps:**
1. Create a `CollectionRepository` with in-memory SQLite
2. Call `list_collections()`
3. Assert the returned list is empty

**Expected result:**
`list_collections()` returns an empty `Vec`.

---

### TC-003: Delete a collection and verify it is removed

**Priority:** High
**Type:** Positive

**Precondition:**
In-memory SQLite with one collection inserted.

**Steps:**
1. Create a collection with a known ID
2. Call `delete_collection(id)`
3. Call `get_collection(id)`

**Expected result:**
`delete_collection` succeeds without error; `get_collection` returns `AppError::NotFound`.

---

### TC-004: Delete a nonexistent collection returns NotFound

**Priority:** High
**Type:** Negative

**Precondition:**
In-memory SQLite, no collections inserted.

**Steps:**
1. Call `delete_collection(nonexistent_uuid)`
2. Assert the result is `AppError::NotFound`

**Expected result:**
Deleting a collection that does not exist returns `AppError::NotFound`.

**Test data:**
```
id: "00000000-0000-0000-0000-000000000000"
```

---

### TC-005: Create a collection with a duplicate name returns error

**Priority:** High
**Type:** Negative

**Precondition:**
In-memory SQLite with one collection named "Existing" already inserted.

**Steps:**
1. Try to create another collection with name "Existing"
2. Assert the result is an error containing "already exists"

**Expected result:**
Creating a duplicate collection name returns `AppError::BadRequest` with a message indicating the collection already exists.

---

### TC-006: Update collection name

**Priority:** High
**Type:** Positive

**Precondition:**
In-memory SQLite with one collection inserted.

**Steps:**
1. Insert a collection with name "Old Name"
2. Call `update_collection(id, &Some("New Name".into()), &None)`
3. Call `get_collection(id)`
4. Assert the name is "New Name"
5. Assert the description is unchanged

**Expected result:**
The collection name is updated; other fields (description) remain unchanged.

---

### TC-007: Update collection description only

**Priority:** High
**Type:** Positive

**Precondition:**
In-memory SQLite with one collection inserted.

**Steps:**
1. Insert a collection with name "My Collection" and description "Old description"
2. Call `update_collection(id, &None, &Some("New description".into()))`
3. Call `get_collection(id)`
4. Assert the description is "New description"
5. Assert the name is still "My Collection"

**Expected result:**
Only the description is updated; the name remains unchanged.

---

### TC-008: Update nonexistent collection returns NotFound

**Priority:** High
**Type:** Negative

**Precondition:**
In-memory SQLite, no collections inserted.

**Steps:**
1. Call `update_collection(nonexistent_uuid, &Some("Name".into()), &None)`
2. Assert the result is `AppError::NotFound`

**Expected result:**
Updating a collection that does not exist returns `AppError::NotFound`.

---

### TC-009: Rename to an existing name returns error

**Priority:** High
**Type:** Negative

**Precondition:**
In-memory SQLite with two collections: "Alpha" and "Beta".

**Steps:**
1. Try to rename "Beta" to "Alpha"
2. Assert the result is `AppError::BadRequest` with a message about duplicate name

**Expected result:**
Renaming a collection to a name that already exists returns an error.

---

### TC-010: CollectionManager renders collection cards

**Priority:** High
**Type:** Positive

**Precondition:**
Pinia store initialized with 2 collections.

**Steps:**
1. Mount `CollectionManager` with mocked store having 2 collections
2. Assert 2 collection card buttons are rendered
3. Assert collection names are displayed

**Expected result:**
The component renders all collections with their names visible.

**Test data:**
```
collections: [
  { id: "id-1", name: "Docs", description: "Technical docs", document_count: 3, created_at: "..." },
  { id: "id-2", name: "Manuals", description: null, document_count: 0, created_at: "..." }
]
```

---

### TC-011: CollectionManager shows empty state

**Priority:** Low
**Type:** Positive

**Precondition:**
Pinia store initialized with empty collections list.

**Steps:**
1. Mount `CollectionManager` with mocked store having 0 collections
2. Assert "No collections yet." text is visible
3. Assert hint text about creating a collection is visible

**Expected result:**
The empty state with instructions is displayed when there are no collections.

---

### TC-012: Create dialog opens on "+ New" click

**Priority:** High
**Type:** Positive

**Precondition:**
Pinia store initialized.

**Steps:**
1. Mount `CollectionManager`
2. Find the "+ New" button
3. Click it
4. Assert a dialog with title "Create Collection" is visible

**Expected result:**
Clicking "+ New" opens the create collection dialog.

---

### TC-013: Create form submits and calls store.createCollection

**Priority:** High
**Type:** Positive

**Precondition:**
Pinia store initialized with `createCollection` mocked to return a new collection.

**Steps:**
1. Mount `CollectionManager`
2. Click "+ New" to open create dialog
3. Fill in the name field with "New Collection"
4. Click the confirm/create button
5. Assert `createCollection` was called with `{ name: "New Collection", description: "" }`

**Expected result:**
The form submission calls the store's `createCollection` action with the correct data.

---

### TC-014: Delete dialog opens on 🗑 click

**Priority:** High
**Type:** Positive

**Precondition:**
Pinia store initialized with 1 collection.

**Steps:**
1. Mount `CollectionManager`
2. Find the 🗑 delete button on the collection card
3. Click it
4. Assert a confirmation dialog is visible with the collection name

**Expected result:**
Clicking the delete button opens a confirmation dialog showing the collection name.

---

### TC-015: Delete confirmation calls store.deleteCollection

**Priority:** High
**Type:** Positive

**Precondition:**
Pinia store initialized with 1 collection, delete dialog open.

**Steps:**
1. Click the delete confirm button in the dialog
2. Assert `deleteCollection` was called with the correct collection ID

**Expected result:**
Confirming deletion calls the store's `deleteCollection` action with the correct ID.

---

### TC-016: Rename dialog opens on ✏️ click

**Priority:** High
**Type:** Positive

**Precondition:**
Pinia store initialized with 1 collection having name "Original" and description "Original desc".

**Steps:**
1. Mount `CollectionManager`
2. Find the ✏️ edit button on the collection card
3. Click it
4. Assert a dialog with title containing "Rename" or "Edit" is visible
5. Assert the name field is pre-filled with "Original"
6. Assert the description field is pre-filled with "Original desc"

**Expected result:**
Clicking the edit button opens a rename dialog pre-filled with the current collection data.

---

### TC-017: Rename form submits and calls store.updateCollection

**Priority:** High
**Type:** Positive

**Precondition:**
Pinia store initialized with 1 collection, rename dialog open.

**Steps:**
1. Change the name field to "Renamed"
2. Click the confirm/rename button
3. Assert `updateCollection` was called with `{ name: "Renamed", description: "Original desc" }`

**Expected result:**
The rename form calls the store's `updateCollection` action with the updated data.

---

### TC-018: Error toast appears on API failure

**Priority:** Medium
**Type:** Negative

**Precondition:**
Pinia store initialized with `createCollection` mocked to return null (simulating failure).

**Steps:**
1. Mount `CollectionManager`
2. Open create dialog
3. Submit the form
4. Assert a toast message with error styling is visible
5. Assert the toast message contains error details

**Expected result:**
When a store operation fails, a toast notification with the error message is displayed.

---

### TC-019: Success toast appears on create/delete/rename

**Priority:** Medium
**Type:** Positive

**Precondition:**
Pinia store actions mocked to succeed.

**Steps:**
1. Mount `CollectionManager`
2. Create a collection successfully
3. Assert a success toast appears with "Collection created" message
4. Delete a collection successfully
5. Assert a success toast appears with "Collection deleted" message
6. Rename a collection successfully
7. Assert a success toast appears with "Collection renamed" message

**Expected result:**
Success toasts appear after each successful collection operation.
