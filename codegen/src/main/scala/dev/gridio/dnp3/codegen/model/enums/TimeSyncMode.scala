package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model
import dev.gridio.dnp3.codegen.model.{Base10, EnumModel, EnumValue}


object TimeSyncMode {

  private val comments = List(
    "Determines what the master station does when it sees the NEED_TIME iin bit"
  )

  def defaultValue = EnumValue("None", 0, "don't perform a time-sync")

  def apply(): EnumModel = model.EnumModel("TimeSyncMode", comments, EnumModel.UInt8, codes, Some(defaultValue), Base10)

  private val codes = List(
    EnumValue("NonLAN", 1, "synchronize the outstation's time using the non-LAN time sync procedure"),
    EnumValue("LAN", 2, "synchronize the outstation's time using the LAN time sync procedure")
  )

}



