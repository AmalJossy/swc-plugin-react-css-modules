import React from 'react';
import './styles.css';
import _getClassNames$0 from "swc-plugin-react-css-modules/dist/browser/getClassName";
const _styleNameObjMap$0 = {
    "": {
        "visible": "styles__visible_VOQZh",
        "something": "styles__something_NSmsy"
    }
};
const comp = ()=><div className={`something ${_getClassNames$0(foo.anotherThing, _styleNameObjMap$0)}`}/>;