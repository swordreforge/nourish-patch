// The `world` table: one record per scene world the picker tracks (its display
// name + grid cell), keyed by world UUID. Projected from the picker's registry
// (`cell_worlds`/`world_names`) and written at the buffer boundary; on load the
// picker grid is restored and the loader recreates each world with its saved id.
pub mod base;
