from pathlib import Path

import mobase
from PyQt6.QtCore import QCoreApplication


class Form43Checker(mobase.IPluginDiagnose):
    __organizer: mobase.IOrganizer

    def __init__(self):
        super().__init__()

    def init(self, organizer: mobase.IOrganizer):
        self.__organizer = organizer
        return True

    def name(self):
        return "Form 43 Plugin Checker"

    def localizedName(self):
        return "Form 43 Plugin Checker"

    def author(self):
        return "AnyOldName3"

    def description(self):
        return self.tr(
            "Checks plugins (.ESM/.ESP files) to see if any are lower than Form 44 (Skyrim SE)."
        )

    def version(self):
        return mobase.VersionInfo(1, 2, 0, mobase.ReleaseType.PRE_ALPHA)

    def requirements(self):
        return [
            mobase.PluginRequirementFactory.gameDependency("Skyrim Special Edition")
        ]

    def settings(self) -> list[mobase.PluginSetting]:
        return []

    def activeProblems(self) -> list[int]:
        if self.__scanPlugins():
            return [0]
        else:
            return []

    def shortDescription(self, key: int) -> str:
        return self.tr("Form 43 (or lower) plugin detected")

    def fullDescription(self, key: int) -> str:
        pluginList = self.__listPlugins()
        pluginList = [Path(absolutePath).name for absolutePath in pluginList]
        pluginListString = "<br><br>•  " + ("<br>•  ".join(pluginList))
        outputString = self.tr(
            "You have one or more plugins that are not form 44. They are:{0}"
        ).format(pluginListString)
        outputString += "<br><br>"
        outputString += self.tr(
            "Form 43 (or lower) plugins are modules that were made for Skyrim LE (Oldrim) and have not been "
            "properly ported to Skyrim Special Edition, which uses form 44 plugins. This usually results in "
            "parts of the mod not working correctly."
            "<br><br>"
            "To be converted, these plugins simply need to be opened and saved with the SSE Creation Kit "
            "but their presence can be an indication that a mod was not properly ported to SSE and so "
            "can potentially have additional issues."
            "<br><br>"
            "Online guides can have more information on how to correctly convert mods for Skyrim SE.<br>"
        )
        return outputString

    def hasGuidedFix(self, key: int) -> bool:
        return False

    def startGuidedFix(self, key: int) -> None:
        # Maybe we could use xEdit or something to resave the file?
        pass

    def tr(self, value: str):
        return QCoreApplication.translate("Form43Checker", value)

    def __testFile(self, path: str) -> bool | None:
        version = self.__getForm(path)
        if isinstance(version, int):
            return version < 44
        else:
            return None

    def __getForm(self, file: str) -> int | str:
        path = Path(file)
        if path.is_file():
            with path.open(mode="rb") as fp:
                fp.seek(20)
                return int.from_bytes(fp.read(2), byteorder="little")
        else:
            return "invalid"

    def __listInvalidFiles(self):
        for file in self.__organizer.findFiles("", "*.es[pm]"):
            if self.__testFile(file):
                yield file

    def __scanPlugins(self):
        if self.__organizer.managedGame().gameName() != "Skyrim Special Edition":
            return False

        # Return True if there is at least one invalid file:
        return next(self.__listInvalidFiles(), False)

    def __listPlugins(self):
        return [
            "{} (form {})".format(file, self.__getForm(file))
            for file in self.__listInvalidFiles()
        ]


def createPlugin():
    return Form43Checker()
