export native fn system_font_catalog() -> U64 = fonts.system_font_catalog
export native fn catalog_count(read catalog: U64) -> Int = fonts.catalog_count
export native fn catalog_family_name(read catalog: U64, index: Int) -> Str = fonts.catalog_family_name
export native fn catalog_face_name(read catalog: U64, index: Int) -> Str = fonts.catalog_face_name
export native fn catalog_full_name(read catalog: U64, index: Int) -> Str = fonts.catalog_full_name
export native fn catalog_postscript_name(read catalog: U64, index: Int) -> Str = fonts.catalog_postscript_name
export native fn catalog_path(read catalog: U64, index: Int) -> Str = fonts.catalog_path
export native fn catalog_destroy(take catalog: U64) = fonts.catalog_destroy
