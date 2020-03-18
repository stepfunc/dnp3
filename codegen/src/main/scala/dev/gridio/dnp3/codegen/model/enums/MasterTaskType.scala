package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{Base10, EnumModel, EnumValues}


object MasterTaskType {

  private val comments = List("Enumeration of internal tasks")

  def apply(): EnumModel = EnumModel("MasterTaskType", comments, EnumModel.UInt8, codes, None, Base10)

  private val codes = EnumValues.from(
    List(
      "CLEAR_RESTART",
      "DISABLE_UNSOLICITED",
      "ASSIGN_CLASS",
      "STARTUP_INTEGRITY_POLL",
      "NON_LAN_TIME_SYNC",
      "LAN_TIME_SYNC",
      "ENABLE_UNSOLICITED",
      "AUTO_EVENT_SCAN",
      "USER_TASK"
    )
  )

}



