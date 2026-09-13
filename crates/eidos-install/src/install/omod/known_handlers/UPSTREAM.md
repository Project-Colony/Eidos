# Native OMOD handler references

These native implementations translate the finite inlined scripts in
erri120/OMODFramework at commit `05b3d5629124cfa44308b3b78c38959b960b3730`:

- `OMODFramework.Scripting/ScriptHandlers/CSharp/InlinedScripts/DarNifiedUI.cs`
  — DarNified UI 1.3.2, Copyright 2007–2008 DarN; original mod
  <https://www.nexusmods.com/oblivion/mods/10763>.
- `OMODFramework.Scripting/ScriptHandlers/CSharp/InlinedScripts/DarkUIdDarN.cs`
  — DarkUI'd DarN 1.6, made by DarN and modified by gothic251, Copyright
  2007–2008 DarN; original mod <https://www.nexusmods.com/oblivion/mods/11280>.
- `OMODFramework.Scripting/ScriptHandlers/CSharp/InlinedScripts/HorseArmorRevamped.cs`
  — Horse Armor Revamped 1.8; original mod
  <https://www.nexusmods.com/oblivion/mods/46657>.
- `OMODFramework/Oblivion/BSA/BSACreator.cs` — Bethesda BSA name hashes and
  directory layout; the native writer checks bounds and constructs a complete
  uncompressed TES4 archive.

Repository: <https://github.com/erri120/OMODFramework>. Its pinned
`Directory.Build.props` declares `GPL-3.0-only`, and its `LICENSE` contains the
GNU General Public License version 3. Eidos distributes that license in the
repository license file. Keep this attribution with the translated code.

The Rust translations were made in September 2026. They replace WinForms with
shared native prompts, keep decisions separate from owned-stage publication,
validate paths, bounds, source CRCs and known mesh layouts, and implement the
archive generation that was unfinished in the reference helper. Player-name
XML metacharacters are escaped, context plugin names are matched without ASCII
case sensitivity, and intended Oblivion XP exclusions are applied to the final
file map. These are explicit corrections rather than claims of byte-for-byte
behavior for the reference framework itself.

Only fixed Rust handlers run. Received C# is never compiled or executed.
Tests use synthetic data, including independently extracted literal-operation
goldens; they contain no original mod or Bethesda asset bytes.
