/** Configured external tools (shared by Settings and the script-node dialog).
 * `key` is the id stored in `NodeEnv.tools` and referenced by `$tool:<key>`. */
export interface ToolDef {
  key: string;
  label: string;
  arg: string;
  hint: string;
}

export const TOOLS: ToolDef[] = [
  { key: "python", label: "Python", arg: "--version", hint: "python.exe" },
  { key: "pip", label: "pip", arg: "--version", hint: "pip.exe" },
  { key: "tshark", label: "TShark", arg: "--version", hint: "Wireshark 命令行" },
  { key: "sevenzip", label: "7-Zip", arg: "i", hint: "7z.exe" },
  { key: "exiftool", label: "ExifTool", arg: "-ver", hint: "exiftool.exe" },
  { key: "java", label: "Java", arg: "-version", hint: "java.exe" },
];
