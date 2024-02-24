export default (styleNameAttrValue:string, styleModuleImportMap: Record<string, Record<string, string>>) => {
  return styleNameAttrValue
    .split(" ")
    .map((styleNameValue) => {
      if(!styleNameValue) return "";
      const moduleAndStyleName = styleNameValue.split(".");
      const module = moduleAndStyleName[1] ? moduleAndStyleName[0] : "";
      const styleName = moduleAndStyleName[1] || moduleAndStyleName[0];
      if (!styleModuleImportMap[module]) {
        throw new Error(`No css-module import with specifier "${module}" found`);
      }
      return styleModuleImportMap[module][styleName];
    })
    .filter(Boolean)
    .join(" ");
};
