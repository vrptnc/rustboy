import React, {MouseEvent, useEffect, useState} from "react";
import {OAMObject, WebEmulator} from "../../../pkg/rustboy";

import "./object-atlas.scss"

export interface ObjectAtlasProps {
  emulator: WebEmulator | undefined
}

export const ObjectAtlas = ({ emulator }: ObjectAtlasProps) => {

  const [objectInfoIndex, setObjectInfoIndex] = useState<number>()
  const [selectedObject, setSelectedObject] = useState<OAMObject>()

  const enableObjectAtlasRendering = () => {
    emulator?.set_object_atlas_rendering_enabled(true)

    return () => emulator?.set_object_atlas_rendering_enabled(false)
  }

  useEffect(enableObjectAtlasRendering, [])
  useEffect(enableObjectAtlasRendering, [emulator])

  useEffect(() => {
    if (objectInfoIndex != null && emulator != null) {
      const object = emulator.get_object(objectInfoIndex)
      setSelectedObject(object)
    } else {
      setSelectedObject(undefined)
    }

  }, [objectInfoIndex])

  const onMouseMoveInObjectCanvas = (event: MouseEvent) => {
    const infoElement = event.currentTarget as Element
    const infoElementStyle = window.getComputedStyle(infoElement)
    const infoElementBoundingRect = infoElement.getBoundingClientRect()
    const x = Math.round(event.clientX - infoElementBoundingRect.left - parseFloat(infoElementStyle.borderLeftWidth ?? '0'))
    const y = Math.round(event.clientY - infoElementBoundingRect.top - parseFloat(infoElementStyle.borderTopWidth ?? '0'))
    if (x >= 0 && x <= 159 && y >= 0 && y <= 31) {
      const objectIndex = Math.floor(y / 16) * 16 + Math.floor(x / 8)
      setObjectInfoIndex(objectIndex)
    } else {
      setObjectInfoIndex(undefined)
    }
  }

  const onMouseLeaveObjectCanvas = () => {
    setObjectInfoIndex(undefined)
  }

  return <div className="object-atlas">
    <canvas id="object-atlas-canvas" onMouseMove={ onMouseMoveInObjectCanvas } onMouseLeave={ onMouseLeaveObjectCanvas }
            width={ 160 } height={ 32 }></canvas>
    {
      selectedObject ? <div id="object-info-container">
        <div>X: { selectedObject.lcd_x }</div>
        <div>Y: { selectedObject.lcd_y }</div>
        <div>Tile Index: { selectedObject.tile_index }</div>
        <div>Attributes: { `0x${ selectedObject.attributes.value().toString(16) }` }</div>
      </div> : <React.Fragment/>
    }
  </div>
}