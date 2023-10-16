import React, {Fragment, useState} from "react";
import {WebEmulator} from "../../../pkg/rustboy";
import {FaTableCells} from "react-icons/fa6";

import './tab-pane.scss'

export interface TabBarProps {
  emulator: WebEmulator | undefined
}

enum Tab {
  TILE_MEMORY
}

export const TabPane = (props: TabBarProps) => {
  const [activeTab, setActiveTab] = useState<Tab>()

  const getContent = () => {
    if(activeTab === Tab.TILE_MEMORY) {
      return <div>
        <canvas id="tile-canvas" width={ 256 } height={ 192 }></canvas>
      </div>
    }
    return <Fragment/>
  }

  return <div className="tab-pane">
    <div className="content">
      {
        getContent()
      }
    </div>
    <div className="tabs">
      <div className="tab" title="Tile Memory">
        <FaTableCells/>
      </div>
    </div>
  </div>
}