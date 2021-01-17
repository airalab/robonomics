---------------------------
Robonomics Docker Compose
---------------------------

-----------------
First time Run*
-----------------
Run the following command:

	./build-compose.sh

----------------------------
Subsequent Run using docker-compose
----------------------------
Run the following command:

	docker-compose up -d

---------------------------------
To STOP docker-compose container
---------------------------------
Run the following command:
  
	docker-compose stop

-----------------------------------
To REMOVE docker-compose container
-----------------------------------
Run the following command:
 	
	docker-compose rm

*[Note]: this command will detect whether there is a binary file in the target/release location is, there is a limitation of Dockerfile where the file has
be within the same directory and cannot be copied from another location hence we have to manually copy to the current directory prior to using docker-compose.
