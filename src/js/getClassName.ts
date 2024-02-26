export default (
  styleNameAttrValue: string,
  styleModuleImportMap: Record<string, Record<string, string>>
) => {
  return styleNameAttrValue
    .split(" ")
    .map((styleNameValue) => {
      if (!styleNameValue) return "";
      const moduleAndStyleName = styleNameValue.split(".");
      const module = moduleAndStyleName[1] ? moduleAndStyleName[0] : "";
      const styleName = moduleAndStyleName[1] || moduleAndStyleName[0];
      if (!module) {
        if (
          styleModuleImportMap[module] &&
          styleModuleImportMap[module][styleName]
        )
          return styleModuleImportMap[module][styleName];
        // else search in all maps
        const possibleModule = Object.keys(styleModuleImportMap).find(
          (key) => styleModuleImportMap[key][styleName]
        );
        if (possibleModule)
          return styleModuleImportMap[possibleModule][styleName];
      }
      if (!styleModuleImportMap[module]) {
        throw new Error(
          `No css-module import with specifier "${module}" found`
        );
      }
      return styleModuleImportMap[module][styleName];
    })
    .filter(Boolean)
    .join(" ");
};
