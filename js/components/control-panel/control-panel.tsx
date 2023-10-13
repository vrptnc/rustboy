import React from 'react'

import './control-panel.scss'
import {Button, WebEmulator} from "../../../pkg/rustboy";

export const ControlPanel = ({emulator}: {emulator: WebEmulator | undefined}) => {
    const getMouseDownHandler = (button: Button) => () => {
        emulator?.press_button(button)
    }

    const getMouseUpHandler = (button: Button) => () => {
        emulator?.release_button(button)
    }

    return <div className="control-panel">
        <div id="up-button" onMouseDown={ getMouseDownHandler(Button.UP) }
             onMouseUp={ getMouseUpHandler(Button.UP) }></div>
        <div id="down-button" onMouseDown={ getMouseDownHandler(Button.DOWN) }
             onMouseUp={ getMouseUpHandler(Button.DOWN) }></div>
        <div id="left-button" onMouseDown={ getMouseDownHandler(Button.LEFT) }
             onMouseUp={ getMouseUpHandler(Button.LEFT) }></div>
        <div id="right-button" onMouseDown={ getMouseDownHandler(Button.RIGHT) }
             onMouseUp={ getMouseUpHandler(Button.RIGHT) }></div>
        <div id="center-button"></div>
        <div className="action-panel">
            <div id="a-button" className="action-button" onMouseDown={ getMouseDownHandler(Button.A) }
                 onMouseUp={ getMouseUpHandler(Button.A) }>
                <div className="label">A</div>
            </div>
            <div id="b-button" className="action-button" onMouseDown={ getMouseDownHandler(Button.B) }
                 onMouseUp={ getMouseUpHandler(Button.B) }>
                <div className="label">B</div>
            </div>
        </div>
        <div id="start-button" onMouseDown={ getMouseDownHandler(Button.START) }
             onMouseUp={ getMouseUpHandler(Button.START) }></div>
        <div id="select-button" onMouseDown={ getMouseDownHandler(Button.SELECT) }
             onMouseUp={ getMouseUpHandler(Button.SELECT) }></div>
    </div>
}