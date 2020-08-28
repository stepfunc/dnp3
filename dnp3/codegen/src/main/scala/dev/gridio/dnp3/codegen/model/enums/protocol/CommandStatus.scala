package dev.gridio.dnp3.codegen.model.enums.protocol

import dev.gridio.dnp3.codegen.model._

object CommandStatus extends EnumModel {

  override def render: IntRender = IntRender.Base10

  override def captureUnknownValues: Boolean = true

  override def comments : List[String] = List(
    "Enumeration received from an outstation in response to command request"
  )

  override def name: String = "CommandStatus"

  override def values: List[EnumValue] = List(
    EnumValue("Success", 0, "command was accepted, initiated, or queued"),
    EnumValue("Timeout", 1, "command timed out before completing"),
    EnumValue("NoSelect", 2, "command requires being selected before operate, configuration issue"),
    EnumValue("FormatError", 3, "bad control code or timing values"),
    EnumValue("NotSupported", 4, "command is not implemented"),
    EnumValue("AlreadyActive", 5, "command is all ready in progress or its all ready in that mode"),
    EnumValue("HardwareError", 6, "something is stopping the command, often a local/remote interlock"),
    EnumValue("Local", 7, "the function governed by the control is in local only control"),
    EnumValue("TooManyOps", 8, "the command has been done too often and has been throttled"),
    EnumValue("NotAuthorized", 9, "the command was rejected because the device denied it or an RTU intercepted it"),
    EnumValue("AutomationInhibit", 10, "command not accepted because it was prevented or inhibited by a local automation process, such as interlocking logic or synchrocheck"),
    EnumValue("ProcessingLimited", 11, "command not accepted because the device cannot process any more activities than are presently in progress"),
    EnumValue("OutOfRange", 12, "command not accepted because the value is outside the acceptable range permitted for this point"),
    EnumValue("DownstreamLocal", 13, "command not accepted because the outstation is forwarding the request to another downstream device which reported LOCAL"),
    EnumValue("AlreadyComplete", 14, "command not accepted because the outstation has already completed the requested operation"),
    EnumValue("Blocked", 15, "command not accepted because the requested function is specifically blocked at the outstation"),
    EnumValue("Canceled", 16, "command not accepted because the operation was cancelled"),
    EnumValue("BlockedOtherMaster", 17, "command not accepted because another master is communicating with the outstation and has exclusive rights to operate this control point"),
    EnumValue("DownstreamFail", 18, "command not accepted because the outstation is forwarding the request to another downstream device which cannot be reached or is otherwise incapable of performing the request"),
    EnumValue("NonParticipating", 126, "(deprecated) indicates the outstation shall not issue or perform the control operation")
  )


}
