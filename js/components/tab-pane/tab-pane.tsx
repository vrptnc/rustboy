import React, {Fragment, ReactNode, useState} from "react";
import {WebEmulator} from "../../../pkg/rustboy";
import {FaTableCells, FaVectorSquare} from "react-icons/fa6";

import './tab-pane.scss'

export interface TabBarProps {
  emulator: WebEmulator | undefined
}

enum Tab {
  TILE_MEMORY,
  OBJECT_MEMORY
}

interface TabConfig {
  icon: ReactNode
  tab: Tab
  callback: (enabled: boolean) => void
}



export const TabPane = ({emulator}: TabBarProps) => {
  const TABS: Array<TabConfig> = [
    {
      icon: <FaTableCells title="Tile Memory" size={ 40 }/>,
      tab: Tab.TILE_MEMORY,
      callback: (enabled: boolean) => emulator?.set_tile_atlas_rendering_enabled(enabled)
    },
    {
      icon: <FaVectorSquare title="Object Memory" size={ 40 }/>,
      tab: Tab.OBJECT_MEMORY,
      callback: (enabled: boolean) => emulator?.set_object_atlas_rendering_enabled(enabled)
    }
  ]

  const [activeTab, setActiveTab] = useState<Tab>()

  const getContent = () => {
    if (activeTab === Tab.TILE_MEMORY) {
      return <div>
        <canvas id="tile-atlas-canvas" width={ 256 } height={ 192 }></canvas>
      </div>
    } else if (activeTab === Tab.OBJECT_MEMORY) {
      return <div>
        <canvas id="tile-atlas-canvas" width={ 256 } height={ 192 }></canvas>
      </div>
    }
    return <Fragment/>
  }

  const getTab = ({icon, tab, callback}: TabConfig) => <div className={'tab' + (activeTab === tab ? ' active' : '')}
         onClick={() => {
           if (activeTab === tab) {
             setActiveTab(undefined)
             setTimeout(() => callback(false), 0)
             callback(false)
           } else {
             setActiveTab(tab)
             setTimeout(() => callback(true), 0)
           }
         }}>
    {
      icon
    }
    </div>

  const Content = () => (
    <div className="content">
      {
        getContent()
      }
    </div>
  )

  return <div className="tab-pane">
    {
      activeTab != null ? <Content/> : null
    }
    <div className="tabs">
      {
        TABS.map((tabConfig) => getTab(tabConfig))
      }
    </div>
  </div>
}