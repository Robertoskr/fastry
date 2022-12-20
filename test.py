import requests
from datetime import datetime 


def test(n): 
    start = datetime.utcnow() 
    for i in range(n): 
        requests.get('http:///hola/hola/adios')
    print(datetime.utcnow() - start)

test(100)


