        inputs:
          Phono:
          - service: mqtt.publish
            data:
              qos: "0"
              topic: tasmota_17DD9F/cmnd/irsend
              payload: " {\"Protocol\":\"RC5\",\"Bits\":12,\"Data\":\"0xC01\",\"Repeat\":0 }"
          CD:
          - service: mqtt.publish
            data:
              qos: "0"
              topic: tasmota_17DD9F/cmnd/irsend
              payload: " {\"Protocol\":\"RC5\",\"Bits\":12,\"Data\":\"0xC02\",\"Repeat\":0 }"
          Spotify:
          - service: mqtt.publish
            data:
              qos: "0"
              topic: tasmota_17DD9F/cmnd/irsend
              payload: " {\"Protocol\":\"RC5\",\"Bits\":12,\"Data\":\"0xC03\",\"Repeat\":0 }"
          source 4:
          - service: mqtt.publish
            data:
              qos: "0"
              topic: tasmota_17DD9F/cmnd/irsend
              payload: " {\"Protocol\":\"RC5\",\"Bits\":12,\"Data\":\"0xC04\",\"Repeat\":0 }"
          source 5:
          - service: mqtt.publish
            data:
              qos: "0"
              topic: tasmota_17DD9F/cmnd/irsend
              payload: " {\"Protocol\":\"RC5\",\"Bits\":12,\"Data\":\"0xC05\",\"Repeat\":0 }"


        volume_up:
          - alias: "Send Volume up"
            service: mqtt.publish
            data:
              qos: "0"
              topic: tasmota_17DD9F/cmnd/irsend
              payload: " {\"Protocol\":\"RC5\",\"Bits\":12,\"Data\":\"0xC10\",\"Repeat\":0 }"
         
        volume_down:
          - alias: "Send Volume down"
            service: mqtt.publish
            data:
              qos: "0"
              topic: tasmota_17DD9F/cmnd/irsend
              payload: " {\"Protocol\":\"RC5\",\"Bits\":12,\"Data\":\"0xC11\",\"Repeat\":0 }"
