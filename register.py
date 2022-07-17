

#->r /home
def route(application, request):
    print(application, request)

#->r /somenice/others
def other_route(application, request):
    print(application, request)


#->r /some/route/hola
def hola_handler(application, request):
    print(application, request)