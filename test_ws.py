import asyncio
import websockets
import json

async def test_connection():
    uri = "ws://localhost:8000/ws"
    print(f"Attempting to connect to {uri}...")
    try:
        async with websockets.connect(uri) as websocket:
            print(f"Successfully connected to {uri}")
            print("Listening for messages...")
            for _ in range(3):
                message = await websocket.recv()
                print(f"Received data: {json.loads(message)}")
    except Exception as e:
        print(f"Connection failed: {e}")

if __name__ == "__main__":
    asyncio.run(test_connection())
