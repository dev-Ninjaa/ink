export interface WorkspaceInfo {
  readonly isOpen: boolean;
  readonly name: string;
  readonly isMultiRoot: boolean;
  readonly folders: readonly WorkspaceFolderInfo[];
  readonly path?: string;
}

export interface WorkspaceFolderInfo {
  readonly name: string;
  readonly path: string;
}
