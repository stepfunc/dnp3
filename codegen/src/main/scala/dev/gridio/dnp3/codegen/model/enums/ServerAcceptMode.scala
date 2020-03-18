package dev.gridio.dnp3.codegen.model.enums

import dev.gridio.dnp3.codegen.model.{Base10, EnumModel, EnumValues}


object ServerAcceptMode {

  private val comments = List(
    "Describes how TCP/TLS server channels handle new connections when an existing connection is already active"
  )

  def apply(): EnumModel = EnumModel("ServerAcceptMode", comments, EnumModel.UInt8, codes, None, Base10)

  private val codes = EnumValues.from(
    List(
      "CloseNew",
      "CloseExisting"
    )
  )

}



