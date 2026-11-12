import React from 'react';
import lockupOnLight from '../../brand/xd2figma-lockup-horizontal-color-on-light.svg';
import lockupOnDark from '../../brand/xd2figma-lockup-horizontal-color-on-dark.svg';
import symbolOnLight from '../../brand/xd2figma-symbol-color-on-light.svg';
import symbolOnDark from '../../brand/xd2figma-symbol-color-on-dark.svg';

// Se muestra la variante que corresponde al tema (ver .brand en style.css) y, en
// ventanas estrechas, solo el símbolo.
export function Brand() {
  return <div className="brand" role="img" aria-label="XD2Figma">
    <img className="lockup on-light" src={lockupOnLight} alt="" draggable="false"/>
    <img className="lockup on-dark" src={lockupOnDark} alt="" draggable="false"/>
    <img className="symbol on-light" src={symbolOnLight} alt="" draggable="false"/>
    <img className="symbol on-dark" src={symbolOnDark} alt="" draggable="false"/>
  </div>;
}
