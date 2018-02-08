import paho.mqtt.client as mqtt
import time

client = mqtt.Client()
client.connect("iot.eclipse.org")

counter = 0

while True:
    message = "hi {}".format(counter)
    print(message)
    client.publish("testing12345/c", message)
    client.loop()
    counter += 1
    time.sleep(2)

