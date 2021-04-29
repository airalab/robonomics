---------------------------
Robonomics Docker Compose 
---------------------------
Users who use docker-compose to build and start the container. The following instructions assumed that docker-compose has already been installed on your system.
If you do not have compose installed, please refer to this website https://docs.docker.com/compose/install/ .
Also, it is required that you build the robonomics images prior to using these commands otherwise you will receive an error message telling you to build.


-----------------
### First time BUILD and RUN*
-----------------
Run the following command:

	bash build-compose.sh

----------------------------
### Subsequent BUILD and RUN using docker-compose
----------------------------
Run the following command:

	docker-compose up -d

---------------------------------
### To STOP docker-compose container
---------------------------------
Run the following command:
  
	docker-compose stop

-----------------------------------
### To REMOVE docker-compose container
-----------------------------------
Run the following command:
 	
	docker-compose rm -f


---------------------------
Robonomics Docker
---------------------------

For those who prefer the standard docker image call, we have you covered below. Once again we assumed that you already have docker installed on your system. 
If you do not have docker installed, please refer to this website https://docs.docker.com/engine/install/ .

-----------------------------------
### To BUILD and RUN docker container
-----------------------------------
Run the following command:
 	
	bash build-docker.sh

-----------------------------------
### To STOP docker container
-----------------------------------
Run the following command:
 	
	docker stop robonomics

-----------------------------------
### To RE-START docker container
-----------------------------------
Run the following command:
 	
	docker start robonomics

-----------------------------------
### To DELETE docker container
-----------------------------------
Run the following command:
 	
	docker rm -f robonomics



*[Note]: this command will detect whether there is a binary file in the target/release location is, there is a limitation of Dockerfile where the file has
be within the same directory and cannot be copied from another location hence we have to manually copy to the current directory prior to using docker-compose.
