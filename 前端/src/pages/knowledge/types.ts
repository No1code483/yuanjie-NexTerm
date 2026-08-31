export interface KbCategory {
  id: number
  name: string
  parent_id: number | null
  library?: string
  sort_order: number
  created_at: number
}

export interface KbEntry {
  id: number
  category_id: number
  name: string
  path_url: string
  entry_type: string
  created_at: number
  updated_at: number
  is_favorited?: number
  content?: string | null
  source_path?: string | null
  file_exists?: boolean
}

export interface TreeNode {
  type: 'category' | 'entry'
  id: number
  name: string
  categoryId?: number
  pathUrl?: string
  entryType?: string
  updatedAt?: number
  sourcePath?: string
  depth: number
  children: TreeNode[]
}

export interface KbDatabase {
  id: string
  name: string
  created_at: number
}

export type SortMode = 'name_asc' | 'name_desc' | 'time_desc' | 'time_asc'

export interface CategoryCount {
  category_id: number
  count: number
}

export interface KbTrackedPath {
  id: number
  path: string
  category_id: number
  library: string
  last_imported_at: number
  created_at: number
}

export interface KbTemplate {
  id: number
  name: string
  icon: string
  description: string
  entry_type: string
  content: string
}

export interface ConfirmDelete {
  type: 'category' | 'entry' | 'batch'
  id?: number
  name?: string
  subCount?: number
  batchCount?: number
}

export interface KbTag {
  id: number
  name: string
  color: string
  created_at: number
}

export type ViewMode = 'all' | 'favorite' | 'recent' | 'tag'

export interface ScanDirFile {
  name: string
  path: string
  size_bytes: number
  extension: string
  file_type: string
}