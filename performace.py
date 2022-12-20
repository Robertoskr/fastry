from datetime import datetime, timedelta
import requests

def main(): 
    now = datetime.utcnow()

    for i in range(100000): 
        requests.get('http://127.0.0.1/home')
    
    end = datetime.utcnow()

    print(now - end)

        



if __name__ == "__main__": 
    main()
