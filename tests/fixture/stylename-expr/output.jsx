import React from 'react';
import './styles.css';
import getClassName from "swc-plugin-react-css-modules/dist/browser/getClassName";
const _styleNameObjMap = {
    "": {
        "something": "styles__something_NSmsy",
        "visible": "styles__visible_VOQZh"
    }
};
const comp = ()=><div className={`something ${getClassName(foo.anotherThing, _styleNameObjMap)}`}/>;