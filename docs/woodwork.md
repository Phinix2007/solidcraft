# SolidCraft Holz — integrated furniture tools

[Deutsch](woodwork-local.md) · English

This fork includes furniture tools inside the native SolidCraft application. No separate generator is required. Set **Preferences → General → Language → English**, then open **Utilities → Add-ins → Furniture Cabinet**. The same dialog contains the advanced tools and cut list.

## Rectangular cabinets

1. Optionally select an axis-aligned box and choose **Use dimensions of the selected box**. Its dimensions and position are copied once; the reference remains in the document.
2. Set width, height, depth, panel thickness, shelves and vertical dividers. Enable the back panel and two overlay doors as needed. Enter material, density, edge banding and blank allowances.
3. Choose **Create cabinet in document**. Existing geometry is preserved. Each cabinet receives independent parameters, such as `Holz1_Breite`. One Undo step removes the entire generated cabinet; Redo restores it.
4. Use **Refresh cut list** after parameter changes. **Save current cut list as CSV** exports the current dimensions and the original material and edge settings. Missing or renamed bodies produce an error.

While editing dimensions, type the complete value and press Enter or leave the field. For example, type `19` for a 19 mm panel without intermediate clamping to 3 mm.

## Sloped bodies and mitered panels

Choose **Panelize Furniture Body**. Select a convex reference solid with 4–12 planar faces, choose one face as the opening, and enter the panel thickness. **Split reference body into mitered panels** creates a panel for each remaining face. Angle bisectors produce matching miters, including corners other than 90 degrees. Use **Hide reference body** to see the interior.

Supported references include boxes, rotated boxes, wedges and sloped prisms. Curved surfaces, concave solids and reference bodies with cutouts are not supported. Very thin or degenerate references are rejected. This creates the outer panels; shelves are not automatically fitted to sloped interiors.

## Miter existing panels

Choose **Furniture Miter**. Select two overlapping panels and an outer broad planar face on each. You can use two selected model faces or the body and face lists. **Cut miter** cuts both panels at the angle bisector in one Undo step. Parallel faces and selections that do not cut both panels are rejected.

## Free trimming

Choose **Trim Furniture Panel**. Select a panel and either:

- Define a cutting plane by a point and normal, or take a selected planar model face. Choose which side to retain.
- Enable **Subtract boundary body (keep boundary body)** and choose another body as the cutting tool.

A plane cut must leave exactly one connected piece and cannot remove the whole panel. The boundary body remains available after subtraction. Undo restores the previous geometry.

## Cut list and manufacturing limits

The latest cabinet is used by default. Shaped panels use projected bounding dimensions in their panel plane, actual geometric volume, and material density for mass. **ContourCheck** flags parts that need contour or miter inspection; a rectangular cut list does not describe their complete shape. Edge banding is not automatically assigned to generated contour panels.

English exports use English headers, decimal points, semicolon separators, quoted cells and UTF-8 with BOM. German exports keep their German headers and decimal commas. Existing part names and user-entered material names are preserved. Command callers can request `woodwork.bom` with `{"language":"en"}`; omitting the language retains the existing German CSV behavior.

All edits are native timeline features. The cutting planes are fixed when created: changing the reference body later does not recalculate the angle bisectors. Recreate the panelization for a changed reference shape. This is a module compiled into this fork, not an independently loadable plugin.

## Build this fork

```sh
cargo build --release -p solidcraft -p solidcraft-cli
./target/release/solidcraft
```

The official download links in the main README lead to upstream builds. Build the branch containing these tools to use this extension.
